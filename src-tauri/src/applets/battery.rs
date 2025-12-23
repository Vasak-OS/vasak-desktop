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
        let use_sysfs = std::path::Path::new(battery_sysfs_path).exists() || std::path::Path::new(ac_sysfs_path).exists();

        if let Some(path) = battery_path {
             // DBus monitoring
             if use_sysfs {
                 // Even if DBus exists, if sysfs is preferred/available logic from original code?
                 // Original code: if use_sysfs { ... } else { DBus stream }
                 // It seems originally it checked both? 
                 // Actually:
                 // let battery_path = find_battery_path...
                 // let match battery_path { Some(path) => ... None => fallback loop }
                 // THEN it checked use_sysfs. This logic in original file was a bit split.
                 // Let's clean it up.
                 
                 // If we have sysfs, use it (polling), as it might be more reliable or preferred by the original author?
                 // Original code had a block `if use_sysfs { loop poll } else { dbus stream }`.
                 // But wait, `find_battery_path` returning Some didn't prevent `use_sysfs` check.
                 // Let's stick to the original preference: if sysfs exists, use polling.
                 self.run_sysfs_loop(app_handle).await;
             } else {
                 self.run_dbus_loop(app_handle, conn, path).await;
             }
        } else {
             // specific fallback loop from original code (Wait for battery to appear?)
             // actually original code: if battery_path is None -> poll every 5s until it appears?
             // Original: `let battery_path = match battery_path { Some => path, None => loop { ... } }`
             // This blocking behaviour inside `match` was likely wrong or blocking the async task.
             // But since we are IN a task, we can verify.
             
             // Simpler approach: If no battery found, try polling occasionally or just rely on sysfs if available.
             if use_sysfs {
                 self.run_sysfs_loop(app_handle).await;
             } else {
                 // No battery found via UPower and no SysFS.
                 // We can start a slow poller to check if it appears.
                 loop {
                     tokio::time::sleep(Duration::from_secs(60)).await;
                     if let Some(info) = get_battery_info().await {
                          let _ = app_handle.emit("battery-update", &info);
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
            tokio::time::sleep(Duration::from_millis(1000)).await;
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

    // UPower Logic
    let upower_proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower",
        "org.freedesktop.UPower"
    ).await.ok()?;
    
    // First distinct battery check
    let devices: Vec<zbus::zvariant::OwnedObjectPath> = upower_proxy
        .call_method("EnumerateDevices", &())
        .await
        .ok()?
        .body()
        .deserialize()
        .ok()?;
    
    for device_path in devices {
        let device_proxy = zbus::Proxy::new(
            &conn,
            "org.freedesktop.UPower",
            device_path.as_str(),
            "org.freedesktop.UPower.Device"
        ).await.ok()?;
        
        let device_type: u32 = device_proxy.get_property("Type").await.unwrap_or(0);
        if device_type == 2 { 
            return get_battery_info_from_proxy(device_proxy).await;
        }
    }
    
    // Fallback to BAT0
    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower/devices/battery_BAT0",
        "org.freedesktop.UPower.Device"
    ).await.ok()?;
    
    get_battery_info_from_proxy(proxy).await
}

async fn get_battery_info_from_proxy(proxy: zbus::Proxy<'_>) -> Option<BatteryInfo> {
    let is_present: bool = proxy.get_property("IsPresent").await.ok()?;
    let percentage: f64 = proxy.get_property("Percentage").await.unwrap_or(0.0);
    let state: u32 = proxy.get_property("State").await.unwrap_or(0);
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
        time_to_empty: proxy.get_property("TimeToEmpty").await.ok(),
        time_to_full: proxy.get_property("TimeToFull").await.ok(),
        is_present,
        is_charging,
        vendor: proxy.get_property("Vendor").await.ok(),
        model: proxy.get_property("Model").await.ok(),
        technology: proxy.get_property("Technology").await.ok(),
        energy: proxy.get_property("Energy").await.ok(),
        energy_full: proxy.get_property("EnergyFull").await.ok(),
        energy_full_design: proxy.get_property("EnergyFullDesign").await.ok(),
        voltage: proxy.get_property("Voltage").await.ok(),
        temperature: proxy.get_property("Temperature").await.ok(),
        serial: proxy.get_property("Serial").await.ok(),
    })
}

async fn find_battery_path(conn: &Connection) -> Option<String> {
    let upower_proxy = zbus::Proxy::new(
        conn,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower",
        "org.freedesktop.UPower"
    ).await.ok()?;
    
    let devices: Vec<zbus::zvariant::OwnedObjectPath> = upower_proxy
        .call_method("EnumerateDevices", &())
        .await
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
        
        let device_type: u32 = device_proxy.get_property("Type").await.unwrap_or(0);
        if device_type == 2 {
            return Some(device_path.to_string());
        }
    }
    None
}
