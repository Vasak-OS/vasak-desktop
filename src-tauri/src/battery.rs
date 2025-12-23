use tauri::{AppHandle, Emitter};
use zbus::{Connection, MessageStream};
use std::sync::Arc;
use once_cell::sync::OnceCell;
use futures_util::StreamExt;

use crate::structs::BatteryInfo;

static BATTERY_CONN: OnceCell<Arc<Connection>> = OnceCell::new();

pub async fn has_battery() -> bool {
    match get_battery_info().await {
        Some(info) => info.has_battery,
        None => false,
    }
}

pub async fn get_battery_info() -> Option<BatteryInfo> {
    let conn = Connection::system().await.ok()?;
    
    // Primero intentar obtener la lista de dispositivos de UPower
    let upower_proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower",
        "org.freedesktop.UPower"
    ).await.ok()?;
    
    // Obtener todos los dispositivos
    let devices: Vec<zbus::zvariant::OwnedObjectPath> = upower_proxy
        .call_method("EnumerateDevices", &())
        .await
        .ok()?
        .body()
        .deserialize()
        .ok()?;
    
    // Buscar el primer dispositivo de batería
    for device_path in devices {
        let device_proxy = zbus::Proxy::new(
            &conn,
            "org.freedesktop.UPower",
            device_path.as_str(),
            "org.freedesktop.UPower.Device"
        ).await.ok()?;
        
        // Verificar si es una batería
        let device_type: u32 = device_proxy.get_property("Type").await.unwrap_or(0);
        if device_type == 2 { 
            return get_battery_info_from_proxy(device_proxy).await;
        }
    }
    
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
    let time_to_empty: Option<u64> = proxy.get_property("TimeToEmpty").await.ok();
    let time_to_full: Option<u64> = proxy.get_property("TimeToFull").await.ok();
    let is_charging = state == 1;
    let vendor: Option<String> = proxy.get_property("Vendor").await.ok();
    let model: Option<String> = proxy.get_property("Model").await.ok();
    let technology: Option<String> = proxy.get_property("Technology").await.ok();
    let energy: Option<f64> = proxy.get_property("Energy").await.ok();
    let energy_full: Option<f64> = proxy.get_property("EnergyFull").await.ok();
    let energy_full_design: Option<f64> = proxy.get_property("EnergyFullDesign").await.ok();
    let voltage: Option<f64> = proxy.get_property("Voltage").await.ok();
    let temperature: Option<f64> = proxy.get_property("Temperature").await.ok();
    let serial: Option<String> = proxy.get_property("Serial").await.ok();
    Some(BatteryInfo {
        has_battery: is_present,
        percentage,
        state: state_str,
        time_to_empty,
        time_to_full,
        is_present,
        is_charging,
        vendor,
        model,
        technology,
        energy,
        energy_full,
        energy_full_design,
        voltage,
        temperature,
        serial,
    })
}

pub fn start_battery_monitor(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let conn = match Connection::system().await {
            Ok(c) => {
                Arc::new(c)
            },
            Err(e) => {
                return;
            }
        };
        BATTERY_CONN.set(conn.clone()).ok();
        
        let battery_path = find_battery_path(&conn).await;
        let battery_path = match battery_path {
            Some(path) => {
                path
            }
            None => {
                let mut last_info: Option<BatteryInfo> = None;
                
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    
                    if let Some(current_info) = get_battery_info().await {
                        let should_emit = match &last_info {
                            None => true,
                            Some(last) => {
                                last.percentage != current_info.percentage || 
                                last.is_charging != current_info.is_charging ||
                                last.state != current_info.state
                            }
                        };
                        
                        if should_emit {
                            if let Err(e) = app_handle.emit("battery-update", &current_info) {
                                println!("Failed to emit battery-update: {}", e);
                            }
                            last_info = Some(current_info);
                        }
                    }
                }
            }
        };
        
        use std::path::Path;
        use std::time::Duration;
        
        let battery_sysfs_path = "/sys/class/power_supply/BAT0";
        let ac_sysfs_path = "/sys/class/power_supply/AC0";
        
        let use_sysfs = Path::new(battery_sysfs_path).exists() || Path::new(ac_sysfs_path).exists();
        
        if use_sysfs {
            
            let mut last_info: Option<BatteryInfo> = None;
            
            loop {
                tokio::time::sleep(Duration::from_millis(1000)).await; // Check every second
                
                if let Some(current_info) = get_battery_info().await {
                    let should_emit = match &last_info {
                        None => true,
                        Some(last) => {
                            (last.percentage - current_info.percentage).abs() > 0.1 || // Cambio de más de 0.1%
                            last.is_charging != current_info.is_charging ||
                            last.state != current_info.state
                        }
                    };
                    
                    if should_emit {
                        if let Err(e) = app_handle.emit("battery-update", &current_info) {
                            println!("Failed to emit battery-update: {}", e);
                        }
                        last_info = Some(current_info);
                    }
                }
            }
        } else {
            let mut stream = MessageStream::from(&*conn);
            
            while let Some(msg_result) = stream.next().await {
                match msg_result {
                    Ok(msg) => {
                        let header = msg.header();
                        
                        if let Some(interface) = header.interface() {
                            if let Some(member) = header.member() {
                                if let Some(path) = header.path() {
                                    
                                    if interface.as_str() == "org.freedesktop.DBus.Properties" &&
                                       member.as_str() == "PropertiesChanged" &&
                                       path.as_str() == battery_path {
                                        if let Some(info) = get_battery_info().await {
                                            if let Err(e) = app_handle.emit("battery-update", &info) {
                                                println!("Failed to emit battery-update: {}", e);
                                            }
                                        } else {
                                            println!("Failed to get battery info after PropertiesChanged");
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error receiving D-Bus message: {}", e);
                    }
                }
            }
        }
    });
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