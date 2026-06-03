use super::Applet;
use crate::structs::BatteryInfo;
use async_trait::async_trait;
use futures_util::StreamExt;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use zbus::{Connection, MessageStream};

pub struct BatteryApplet;

// Keep this global for the static methods (used by commands)
static BATTERY_CONN: Mutex<Option<Arc<Connection>>> = Mutex::new(None);
static BATTERY_DEVICE_PATH: Mutex<Option<String>> = Mutex::new(None);

#[async_trait]
impl Applet for BatteryApplet {
    fn name(&self) -> &'static str {
        "battery"
    }

    async fn start(&self, app_handle: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        let conn = match Connection::system().await {
            Ok(c) => Arc::new(c),
            Err(e) => return Err(Box::new(e)),
        };
        *BATTERY_CONN.lock().unwrap() = Some(conn.clone());

        let battery_path = find_battery_path(&conn).await;
        
        // Define paths for sysfs fallback
        let battery_sysfs_path = "/sys/class/power_supply/BAT0";
        let ac_sysfs_path = "/sys/class/power_supply/AC0";
        let has_sysfs = std::path::Path::new(battery_sysfs_path).exists() || std::path::Path::new(ac_sysfs_path).exists();

        // Emit initial state
        if let Some(info) = get_battery_info().await {
            let _ = app_handle.emit("battery-update", &info);
        }

        if let Some(path) = battery_path {
             // Cache the path for future lookups
             BATTERY_DEVICE_PATH.lock().unwrap().replace(path.clone());
             
             // Prefer DBus events if available
             self.run_dbus_loop(app_handle, conn, path).await;
        } else {
             // Fallback to polling if UPower device not found
             // If we have sysfs files, we assume battery exists and we should poll
             // Note: get_battery_info currently relies on UPower, so if UPower is missing
             // this might fail. Ideally we should have a pure sysfs reader here.
             // For now, we reduce poll frequency to 2s to save CPU.
             if has_sysfs {
                 self.run_sysfs_loop(app_handle).await;
             } else {
                 // No battery found via UPower and no SysFS.
                 // Monitor occasionally in case it appears (e.g. plugged in)
                 loop {
                     tokio::time::sleep(Duration::from_secs(60)).await;
                     if let Some(info) = get_battery_info().await {
                          let _ = app_handle.emit("battery-update", &info);
                          // If we found it, we should probably switch mode, but simple periodic check is fine
                     }
                 }
             }
        }

        Ok(())
    }
}

impl BatteryApplet {
    async fn run_sysfs_loop(&self, app_handle: AppHandle) {
        let mut last_info: Option<BatteryInfo> = None;
        loop {
            // Increased polling interval to 2s for optimization
            tokio::time::sleep(Duration::from_secs(2)).await;
            
            if let Some(current_info) = get_battery_info().await {
                let should_emit = match &last_info {
                    None => true,
                    Some(last) => {
                        (last.percentage - current_info.percentage).abs() > 0.1 || 
                        last.is_charging != current_info.is_charging ||
                        last.state != current_info.state
                    }
                };
                
                if should_emit {
                    let _ = app_handle.emit("battery-update", &current_info);
                    last_info = Some(current_info);
                }
            }
        }
    }

    async fn run_dbus_loop(&self, app_handle: AppHandle, mut conn: Arc<Connection>, mut path: String) {
        let mut reconnect_attempts = 0u32;
        let max_reconnects = 5;
        
        loop {
            match self.monitor_dbus_with_reconnect(&app_handle, conn.clone(), path.clone(), reconnect_attempts).await {
                Ok(_) => {
                    log::info!("[battery] D-Bus monitor ended normally");
                    break;
                }
                Err(e) => {
                    reconnect_attempts += 1;
                    if reconnect_attempts >= max_reconnects {
                        log::error!("[battery] Max reconnection attempts reached: {}", e);
                        let _ = app_handle.emit("dbus-status", serde_json::json!({
                            "service": "battery",
                            "status": "failed",
                            "message": "No se pudo conectar a UPower"
                        }));
                        // Fallback to polling
                        self.run_sysfs_loop(app_handle).await;
                        break;
                    }
                    log::warn!("[battery] Connection lost (attempt {}): {}. Reconnecting...", reconnect_attempts, e);
                    let _ = app_handle.emit("dbus-status", serde_json::json!({
                        "service": "battery",
                        "status": "reconnecting",
                        "attempt": reconnect_attempts
                    }));
                    tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(reconnect_attempts.min(3)))).await;

                    // Create fresh connection and update caches
                    if let Ok(c) = Connection::system().await {
                        let new_conn = Arc::new(c);
                        *BATTERY_CONN.lock().unwrap() = Some(new_conn.clone());
                        conn = new_conn;
                    }
                    if let Some(p) = find_battery_path(&conn).await {
                        BATTERY_DEVICE_PATH.lock().unwrap().replace(p.clone());
                        path = p;
                    }
                }
            }
        }
    }

    async fn monitor_dbus_with_reconnect(&self, app_handle: &AppHandle, conn: Arc<Connection>, path: String, attempt: u32) -> Result<(), String> {
        if attempt > 0 {
            log::info!("[battery] Reconnected successfully after {} attempts", attempt);
            let _ = app_handle.emit("dbus-status", serde_json::json!({
                "service": "battery",
                "status": "connected"
            }));
        }

        let mut last_info: Option<BatteryInfo> = None;
        let mut stream = MessageStream::from(conn.as_ref());

        loop {
            tokio::select! {
                biased;

                msg_result = stream.next() => {
                    let msg = match msg_result {
                        Some(Ok(msg)) => msg,
                        Some(Err(e)) => {
                            log::error!("[battery] Stream error: {}", e);
                            return Err(format!("D-Bus stream error: {}", e));
                        }
                        None => {
                            return Err("D-Bus connection closed".to_string());
                        }
                    };

                    let header = msg.header();
                    if let (Some(interface), Some(member), Some(obj_path)) = (header.interface(), header.member(), header.path()) {
                        if interface.as_str() == "org.freedesktop.DBus.Properties" &&
                           member.as_str() == "PropertiesChanged" &&
                           obj_path.as_str() == path {
                               let info = get_battery_info().await;
                               Self::emit_if_changed(app_handle, &mut last_info, &info).await;
                        }
                    }
                }

                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    let info = get_battery_info().await;
                    Self::emit_if_changed(app_handle, &mut last_info, &info).await;
                }
            }
        }
    }

    async fn emit_if_changed(app_handle: &AppHandle, last_info: &mut Option<BatteryInfo>, current: &Option<BatteryInfo>) {
        let should_emit = match (last_info.as_ref(), current) {
            (_, None) => false,
            (None, Some(_)) => true,
            (Some(last), Some(cur)) => {
                (last.percentage - cur.percentage).abs() > 0.5
                    || last.is_charging != cur.is_charging
                    || last.state != cur.state
            }
        };

        if should_emit {
            let _ = app_handle.emit("battery-update", current);
            *last_info = current.clone();
        }
    }
}

// Public Helpers (Commands)
pub async fn has_battery() -> bool {
    match get_battery_info().await {
        Some(info) => info.has_battery,
        None => false,
    }
}

pub async fn get_battery_info() -> Option<BatteryInfo> {
    // Try cached connection, or create a new one
    let cached_conn = BATTERY_CONN.lock().unwrap().clone();
    let conn = match cached_conn {
        Some(c) => c,
        None => Connection::system().await.ok().map(Arc::new)?,
    };

    // Try cached path, or search for it
    let cached_path = BATTERY_DEVICE_PATH.lock().unwrap().clone();
    let device_path = match cached_path {
        Some(p) => p,
        None => find_battery_path(&conn).await
            .unwrap_or_else(|| "/org/freedesktop/UPower/devices/battery_BAT0".to_string()),
    };

    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.UPower",
        device_path.as_str(),
        "org.freedesktop.UPower.Device"
    ).await.ok()?;

    let result = get_battery_info_from_proxy(proxy).await;
    if result.is_some() {
        return result;
    }

    // Stale connection — create fresh one and update cache
    if let Ok(new_conn) = Connection::system().await {
        let new_conn = Arc::new(new_conn);
        *BATTERY_CONN.lock().unwrap() = Some(new_conn.clone());

        let new_path = find_battery_path(&new_conn).await
            .unwrap_or_else(|| device_path.clone());
        BATTERY_DEVICE_PATH.lock().unwrap().replace(new_path.clone());

        let proxy = zbus::Proxy::new(
            &new_conn,
            "org.freedesktop.UPower",
            new_path.as_str(),
            "org.freedesktop.UPower.Device"
        ).await.ok()?;
        return get_battery_info_from_proxy(proxy).await;
    }

    None
}

async fn get_battery_info_from_proxy(proxy: zbus::Proxy<'_>) -> Option<BatteryInfo> {
    let is_present: bool = tokio::time::timeout(Duration::from_millis(300), proxy.get_property("IsPresent"))
        .await
        .ok()?
        .ok()?;
    let percentage: f64 = tokio::time::timeout(Duration::from_millis(300), proxy.get_property("Percentage"))
        .await
        .ok()
        .and_then(|r| r.ok())
        .unwrap_or(0.0);
    let state: u32 = tokio::time::timeout(Duration::from_millis(300), proxy.get_property("State"))
        .await
        .ok()
        .and_then(|r| r.ok())
        .unwrap_or(0);
    let state_str = match state {
        1 => "Charging",
        2 => "Discharging",
        3 => "Empty",
        4 => "FullyCharged",
        5 => "PendingCharge",
        6 => "PendingDischarge",
        _ => "Unknown",
    }.to_string();
    
    // Additional properties...
    let is_charging = state == 1;
    
    Some(BatteryInfo {
        has_battery: is_present,
        percentage,
        state: state_str,
        time_to_empty: tokio::time::timeout(Duration::from_millis(300), proxy.get_property("TimeToEmpty")).await.ok().and_then(|r| r.ok()),
        time_to_full: tokio::time::timeout(Duration::from_millis(300), proxy.get_property("TimeToFull")).await.ok().and_then(|r| r.ok()),
        is_present,
        is_charging,
        vendor: tokio::time::timeout(Duration::from_millis(300), proxy.get_property("Vendor")).await.ok().and_then(|r| r.ok()),
        model: tokio::time::timeout(Duration::from_millis(300), proxy.get_property("Model")).await.ok().and_then(|r| r.ok()),
        technology: tokio::time::timeout(Duration::from_millis(300), proxy.get_property("Technology")).await.ok().and_then(|r| r.ok()),
        energy: tokio::time::timeout(Duration::from_millis(300), proxy.get_property("Energy")).await.ok().and_then(|r| r.ok()),
        energy_full: tokio::time::timeout(Duration::from_millis(300), proxy.get_property("EnergyFull")).await.ok().and_then(|r| r.ok()),
        energy_full_design: tokio::time::timeout(Duration::from_millis(300), proxy.get_property("EnergyFullDesign")).await.ok().and_then(|r| r.ok()),
        voltage: tokio::time::timeout(Duration::from_millis(300), proxy.get_property("Voltage")).await.ok().and_then(|r| r.ok()),
        temperature: tokio::time::timeout(Duration::from_millis(300), proxy.get_property("Temperature")).await.ok().and_then(|r| r.ok()),
        serial: tokio::time::timeout(Duration::from_millis(300), proxy.get_property("Serial")).await.ok().and_then(|r| r.ok()),
    })
}

async fn find_battery_path(conn: &Connection) -> Option<String> {
    let upower_proxy = zbus::Proxy::new(
        conn,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower",
        "org.freedesktop.UPower"
    ).await.ok()?;
    
    let devices: Vec<zbus::zvariant::OwnedObjectPath> = tokio::time::timeout(
        Duration::from_millis(500),
        upower_proxy.call_method("EnumerateDevices", &())
    )
        .await
        .ok()?
        .ok()?
        .body()
        .deserialize()
        .ok()?;
    
    for device_path in devices {
        let device_proxy = zbus::Proxy::new(
            conn,
            "org.freedesktop.UPower",
            device_path.as_str(),
            "org.freedesktop.UPower.Device"
        ).await.ok()?;
        
        let device_type: u32 = tokio::time::timeout(Duration::from_millis(300), device_proxy.get_property("Type"))
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or(0);
        if device_type == 2 {
            return Some(device_path.to_string());
        }
    }
    None
}
