//! Notification client.
//!
//! The freedesktop notification server now lives in `vasak-flare-daemon`. This
//! module is a thin D-Bus client of that daemon: it reads the history/unread
//! list from `org.vasak.Notifications`, forwards mark-read/clear/action calls,
//! sends notifications through `org.freedesktop.Notifications`, and re-emits a
//! `notification-delta` event to the frontend whenever the daemon signals a
//! change (no polling).

use crate::logger::{log_error, log_info};
use crate::structs::{Notification, NotificationUrgency};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use zbus::zvariant::Value;
use zbus::{Connection, Proxy};

const FLARE_DEST: &str = "org.vasak.Notifications";
const FLARE_PATH: &str = "/org/vasak/Notifications";
const FLARE_IFACE: &str = "org.vasak.Notifications";
const FDN_DEST: &str = "org.freedesktop.Notifications";
const FDN_PATH: &str = "/org/freedesktop/Notifications";
const FDN_IFACE: &str = "org.freedesktop.Notifications";

static APP_HANDLE: LazyLock<Arc<RwLock<Option<AppHandle>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));
static CONNECTION: LazyLock<Arc<RwLock<Option<Connection>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

// Matches vasak-flare-daemon's StoredNotification JSON.
#[derive(Deserialize)]
struct FlareNotification {
    id: i64,
    #[allow(dead_code)]
    notif_id: u32,
    app_name: String,
    app_icon: String,
    summary: String,
    body: String,
    urgency: u8,
    actions: Vec<String>,
    created_at: i64,
    read: bool,
}

/// Lo que va al frontend cada vez que el demonio avisa un cambio.
///
/// Es una foto entera, en **un solo** evento, y eso es a propósito. Antes eran
/// dos —`Cleared` y después `BatchUpdate` con todo de nuevo—, y como cada
/// `emit` es un mensaje aparte, el frontend alcanzaba a dibujar la lista vacía
/// entre uno y otro: borrar una sola notificación desmontaba las demás y las
/// volvía a montar, así que todas repetían la animación de entrada. Con un
/// evento la lista se reemplaza de una, Vue compara por clave y sólo se anima
/// lo que de verdad entró o salió.
#[derive(Serialize, Clone)]
#[serde(tag = "action", rename_all = "snake_case")]
enum NotificationDelta {
    Snapshot { items: Vec<Notification> },
}

fn map(f: FlareNotification) -> Notification {
    Notification {
        id: f.id as u32,
        app_name: f.app_name,
        summary: f.summary,
        body: f.body,
        app_icon: f.app_icon,
        timestamp: f.created_at as u64,
        seen: f.read,
        urgency: match f.urgency {
            0 => NotificationUrgency::Low,
            2 => NotificationUrgency::Critical,
            _ => NotificationUrgency::Normal,
        },
        actions: f.actions,
        hints: HashMap::new(),
    }
}

async fn connection() -> Result<Connection, String> {
    if let Some(c) = CONNECTION.read().await.as_ref() {
        return Ok(c.clone());
    }
    let c = Connection::session()
        .await
        .map_err(|e| format!("No se pudo conectar al bus de sesión: {e}"))?;
    *CONNECTION.write().await = Some(c.clone());
    Ok(c)
}

async fn fetch_all() -> Result<Vec<Notification>, String> {
    let conn = connection().await?;
    let reply = conn
        .call_method(Some(FLARE_DEST), FLARE_PATH, Some(FLARE_IFACE), "GetAll", &(0i64,))
        .await
        .map_err(|e| format!("flare GetAll falló: {e}"))?;
    let body = reply.body();
    let json: String = body.deserialize().map_err(|e| e.to_string())?;
    let items: Vec<FlareNotification> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(items.into_iter().map(map).collect())
}

pub async fn get_notifications() -> Result<Vec<Notification>, String> {
    fetch_all().await
}

pub async fn remove_notification(id: u32) -> Result<bool, String> {
    let conn = connection().await?;
    conn.call_method(Some(FLARE_DEST), FLARE_PATH, Some(FLARE_IFACE), "Clear", &(id as i64,))
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}

pub async fn clear_all_notifications() -> Result<u32, String> {
    let count = fetch_all().await.map(|v| v.len() as u32).unwrap_or(0);
    let conn = connection().await?;
    conn.call_method(Some(FLARE_DEST), FLARE_PATH, Some(FLARE_IFACE), "ClearAll", &())
        .await
        .map_err(|e| e.to_string())?;
    Ok(count)
}

pub async fn invoke_action(id: u32, action_key: String) -> Result<(), String> {
    let conn = connection().await?;
    conn.call_method(
        Some(FLARE_DEST),
        FLARE_PATH,
        Some(FLARE_IFACE),
        "InvokeAction",
        &(id as i64, action_key),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn send_system_notification(
    summary: String,
    body: Option<String>,
    urgency: Option<String>,
) -> Result<String, String> {
    let conn = connection().await?;
    let urgency_u8: u8 = match urgency.as_deref() {
        Some("low") => 0,
        Some("critical") => 2,
        _ => 1,
    };
    let mut hints: HashMap<&str, Value> = HashMap::new();
    hints.insert("urgency", Value::U8(urgency_u8));
    let actions: Vec<&str> = Vec::new();
    let args = (
        "VasakOS",
        0u32,
        "",
        summary.as_str(),
        body.as_deref().unwrap_or(""),
        actions,
        hints,
        -1i32,
    );
    conn.call_method(Some(FDN_DEST), FDN_PATH, Some(FDN_IFACE), "Notify", &args)
        .await
        .map_err(|e| e.to_string())?;
    Ok("Notification sent".to_string())
}

/// Lee la lista actual y la manda entera al frontend, en un solo evento.
async fn emit_current() {
    let items = match fetch_all().await {
        Ok(v) => v,
        Err(e) => {
            log_error(&format!("No se pudo leer notificaciones de flare: {e}"));
            return;
        }
    };
    if let Some(app) = APP_HANDLE.read().await.as_ref() {
        let _ = app.emit("notification-delta", NotificationDelta::Snapshot { items });
    }
}

async fn listen_for_changes() -> Result<(), String> {
    let conn = connection().await?;
    let proxy = Proxy::new(&conn, FLARE_DEST, FLARE_PATH, FLARE_IFACE)
        .await
        .map_err(|e| e.to_string())?;
    let mut stream = proxy
        .receive_signal("Changed")
        .await
        .map_err(|e| e.to_string())?;
    while stream.next().await.is_some() {
        emit_current().await;
    }
    Ok(())
}

/// Drops the cached D-Bus connection so the next use reconnects.
async fn reset_connection() {
    *CONNECTION.write().await = None;
}

pub async fn initialize_app_handle(app_handle: AppHandle) {
    *APP_HANDLE.write().await = Some(app_handle);

    tokio::spawn(async {
        // Follows the daemon's Changed signal, reconnecting when it goes away.
        //
        // The stream used to be awaited in a bare `while let`: when the daemon
        // restarted the stream simply ended, the loop returned Ok(()), and
        // notifications silently froze for the rest of the session — with the
        // D-Bus connection cached forever, nothing would have recovered even if
        // something had retried.
        let mut delay = std::time::Duration::from_secs(1);

        loop {
            emit_current().await;

            match listen_for_changes().await {
                Ok(()) => {
                    log_info("El demonio de notificaciones cerró la suscripción; reconectando");
                    delay = std::time::Duration::from_secs(1);
                }
                Err(e) => {
                    log_error(&format!("Suscripción a notificaciones de flare falló: {e}"));
                    // Back off so a daemon that is down doesn't become a spin loop.
                    delay = (delay * 2).min(std::time::Duration::from_secs(30));
                }
            }

            reset_connection().await;
            tokio::time::sleep(delay).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn una(id: u32) -> Notification {
        Notification {
            id,
            app_name: "Telegram".into(),
            app_icon: "telegram".into(),
            summary: "Hola".into(),
            body: String::new(),
            timestamp: 100,
            seen: false,
            urgency: NotificationUrgency::Normal,
            actions: Vec::new(),
            hints: HashMap::new(),
        }
    }

    /// El frontend elige qué hacer mirando `action`, así que el nombre es
    /// contrato y no un detalle del enum.
    #[test]
    fn la_foto_se_serializa_como_snapshot() {
        let json = serde_json::to_value(NotificationDelta::Snapshot {
            items: vec![una(1), una(2)],
        })
        .expect("la foto se serializa");

        assert_eq!(json["action"], "snapshot");
        assert_eq!(json["items"].as_array().map(Vec::len), Some(2));
        assert_eq!(json["items"][0]["id"], 1);
    }

    /// Vaciar la lista es una foto sin nada adentro, no un evento aparte: es
    /// justamente el segundo evento el que hacía que las notificaciones
    /// sobrevivientes repitieran la animación de entrada.
    #[test]
    fn vaciar_es_una_foto_vacia() {
        let json = serde_json::to_value(NotificationDelta::Snapshot { items: Vec::new() })
            .expect("la foto vacía se serializa");

        assert_eq!(json["action"], "snapshot");
        assert_eq!(json["items"].as_array().map(Vec::len), Some(0));
    }
}
