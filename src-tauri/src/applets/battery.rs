use super::Applet;
use crate::structs::BatteryInfo;
use async_trait::async_trait;
use futures_util::StreamExt;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use zbus::{Connection, MessageStream};

pub struct BatteryApplet;

// Keep this global for the static methods (used by commands)
static BATTERY_CONN: OnceCell<Arc<Connection>> = OnceCell::new();
static BATTERY_DEVICE_PATH: OnceCell<String> = OnceCell::new();

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
        BATTERY_CONN.set(conn.clone()).ok();

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
             BATTERY_DEVICE_PATH.set(path.clone()).ok();
             
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

    async fn run_dbus_loop(&self, app_handle: AppHandle, conn: Arc<Connection>, path: String) {
        let mut stream = MessageStream::from(conn.as_ref());
        while let Some(msg_result) = stream.next().await {
            if let Ok(msg) = msg_result {
                let header = msg.header();
                if let (Some(interface), Some(member), Some(obj_path)) = (header.interface(), header.member(), header.path()) {
                    if interface.as_str() == "org.freedesktop.DBus.Properties" &&
                       member.as_str() == "PropertiesChanged" &&
                       obj_path.as_str() == path {
                           // Use cached path logic in get_battery_info
                           if let Some(info) = get_battery_info().await {
                                let _ = app_handle.emit("battery-update", &info);
                           }
                    }
                }
            }
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
    // Try to reuse connection if possible, or create new one (system)
    let conn = if let Some(c) = BATTERY_CONN.get() {
        c.clone()
    } else {
        Connection::system().await.ok().map(Arc::new)?
    };

    // Optimization: Use cached path if available to avoid enumeration
    let device_path = if let Some(path) = BATTERY_DEVICE_PATH.get() {
        path.clone()
    } else {
        // Fallback: search for it (and cache it if found?)
        // find_battery_path caches it? No, find_battery_path is external.
        // Let's just find it.
        find_battery_path(&conn).await.unwrap_or_else(|| "/org/freedesktop/UPower/devices/battery_BAT0".to_string())
    };
    
    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.UPower",
        device_path.as_str(),
        "org.freedesktop.UPower.Device"
    ).await.ok()?;
    
    get_battery_info_from_proxy(proxy).await
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
