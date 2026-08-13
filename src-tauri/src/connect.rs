//! The panel's side of `vasak-connect`: the Android phone service.
//!
//! The service publishes what it can see on the session bus; this reads it and
//! hands it to the panel. Nothing here talks to a phone, to adb or to scrcpy —
//! that all lives in the daemon, which is the only process that knows when a
//! cable comes out.
//!
//! **On the mirrored types.** The contract's home is the `protocol` crate of
//! vasak-connect, and depending on it directly would be the point of having it.
//! It cannot be done yet: that crate derives `Type` from zbus 5's zvariant and
//! this application is on zbus 4, so the two derives are incompatible. The
//! structs below therefore mirror it by hand — if the daemon's contract
//! changes, this file has to change with it, and nothing will remind anybody.
//! Worth revisiting whenever vasak-desktop moves to zbus 5.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use zbus::zvariant::Type;
use zbus::Connection;

use crate::dbus_pool::DbusPool;
use crate::logger;

const SERVICE: &str = "ar.net.vasak.os.Connect";
const PATH: &str = "/ar/net/vasak/os/Connect";

/// A phone the service can currently see.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ConnectDevice {
    pub serial: String,
    pub model: String,
    /// `usb` or `tcp`.
    pub transport: String,
    /// `ready`, `unauthorized`, `connecting` or `offline`.
    pub state: String,
    pub trusted: bool,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ConnectApp {
    pub package: String,
    pub label: String,
    /// Shipped with Android rather than installed by the person. The menu hides
    /// these unless asked: 39 of the 128 apps on a normal phone are things like
    /// "Configuración de Bluetooth", and a list where they are mixed in is not
    /// a menu, it is a haystack.
    pub system: bool,
    /// Path to an icon, or empty — the daemon does not extract them yet.
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ConnectRunningApp {
    pub serial: String,
    pub package: String,
    pub label: String,
    pub pid: u32,
}

/// Borrows the shared session connection.
///
/// Returns `None` rather than an error when the bus is unavailable: every
/// caller here treats "no service" and "no phone" the same way — an empty list
/// — because to the person looking at the panel they are the same thing.
async fn session(app: &AppHandle) -> Option<Connection> {
    app.try_state::<DbusPool>()?.session().await
}

/// Calls a method on the service and deserialises the reply.
async fn call<A, R>(app: &AppHandle, method: &str, args: &A) -> Result<R, String>
where
    A: serde::ser::Serialize + Type,
    R: for<'d> Deserialize<'d> + Type,
{
    let connection = session(app)
        .await
        .ok_or_else(|| "no hay conexión con el bus de sesión".to_string())?;

    let reply = connection
        .call_method(Some(SERVICE), PATH, Some(SERVICE), method, args)
        .await
        .map_err(|err| err.to_string())?;

    reply.body().deserialize().map_err(|err| err.to_string())
}

/// The phones connected right now.
///
/// An empty list is the normal answer: most of the time there is no phone, and
/// the service may not even be running. Neither is worth an error dialog.
#[tauri::command]
pub async fn connect_list_devices(app: AppHandle) -> Vec<ConnectDevice> {
    match call::<(), Vec<ConnectDevice>>(&app, "ListDevices", &()).await {
        Ok(devices) => devices,
        Err(err) => {
            logger::log_info(&format!("vasak-connect: no se pudieron listar dispositivos: {err}"));
            Vec::new()
        }
    }
}

/// The apps installed on a phone.
///
/// The first call for a device takes several seconds — the phone has to walk
/// its whole package database — and the daemon caches the result afterwards.
/// `refresh` forces it to look again, which is what the menu's reload button
/// should do: nothing tells us when somebody installs an app on the phone.
#[tauri::command]
pub async fn connect_list_apps(
    app: AppHandle,
    serial: String,
    refresh: bool,
) -> Result<Vec<ConnectApp>, String> {
    call(&app, "ListApps", &(serial, refresh)).await
}

/// Opens an app in its own window.
#[tauri::command]
pub async fn connect_launch_app(
    app: AppHandle,
    serial: String,
    package: String,
) -> Result<u32, String> {
    call(&app, "LaunchApp", &(serial, package)).await
}

#[tauri::command]
pub async fn connect_stop_app(
    app: AppHandle,
    serial: String,
    package: String,
) -> Result<bool, String> {
    call(&app, "StopApp", &(serial, package)).await
}

/// The app windows open right now.
#[tauri::command]
pub async fn connect_list_running(app: AppHandle) -> Vec<ConnectRunningApp> {
    call::<(), Vec<ConnectRunningApp>>(&app, "ListRunning", &())
        .await
        .unwrap_or_default()
}

/// Forwards the service's signals to the panel as Tauri events.
///
/// Without this the panel would have to poll to notice a phone, which is the
/// cost this whole design exists to avoid: the daemon already knows the instant
/// udev tells it, and the panel should learn at the same moment.
///
/// `DeviceChanged` matters as much as `DeviceAdded`: a phone shows up as
/// `unauthorized` until the person taps "Allow USB debugging", and it becomes
/// usable without ever being unplugged.
pub async fn watch_signals(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let Some(connection) = session(&app).await else {
        logger::log_info("vasak-connect: sin bus de sesión, no se escuchan señales");
        return Ok(());
    };

    // One match rule for the whole interface rather than four: fewer round
    // trips to the bus, and no chance of half the signals being subscribed.
    let rule = format!(
        "type='signal',sender='{SERVICE}',path='{PATH}',interface='{SERVICE}'"
    );

    let proxy = zbus::fdo::DBusProxy::new(&connection).await?;
    proxy
        .add_match_rule(rule.as_str().try_into()?)
        .await
        .map_err(|err| format!("no se pudo suscribir a {SERVICE}: {err}"))?;

    tauri::async_runtime::spawn(async move {
        use futures_util::StreamExt;

        let mut stream = zbus::MessageStream::from(connection);
        while let Some(Ok(message)) = stream.next().await {
            let header = message.header();
            if header.interface().map(|i| i.as_str()) != Some(SERVICE) {
                continue;
            }
            let Some(member) = header.member().map(|m| m.as_str().to_owned()) else {
                continue;
            };

            // The payload is passed straight through as JSON so the panel can
            // update a single device instead of re-listing everything.
            match member.as_str() {
                "DeviceAdded" | "DeviceChanged" => {
                    if let Ok(device) = message.body().deserialize::<ConnectDevice>() {
                        let event = if member == "DeviceAdded" {
                            "connect-device-added"
                        } else {
                            "connect-device-changed"
                        };
                        let _ = app.emit(event, device);
                    }
                }
                "DeviceRemoved" => {
                    if let Ok(serial) = message.body().deserialize::<String>() {
                        let _ = app.emit("connect-device-removed", serial);
                    }
                }
                "AppClosed" => {
                    if let Ok((serial, package)) = message.body().deserialize::<(String, String)>()
                    {
                        let _ = app.emit("connect-app-closed", (serial, package));
                    }
                }
                _ => {}
            }
        }
        logger::log_info("vasak-connect: se cortó el flujo de señales");
    });

    Ok(())
}
