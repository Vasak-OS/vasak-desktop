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
        _none => false,
    }
}

pub async fn get_battery_info() -> Option<BatteryInfo> {
    let conn = Connection::system().await.ok()?;
    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower/devices/battery_BAT0",
        "org.freedesktop.UPower.Device"
    ).await.ok()?;
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
            Ok(c) => Arc::new(c),
            Err(_) => return,
        };
        BATTERY_CONN.set(conn.clone()).ok();
        let _proxy = match zbus::Proxy::new(
            &conn,
            "org.freedesktop.UPower",
            "/org/freedesktop/UPower/devices/battery_BAT0",
            "org.freedesktop.UPower.Device"
        ).await {
            Ok(p) => p,
            Err(_) => return,
        };
        let mut stream = MessageStream::from(&*conn);
        while let Some(msg_result) = stream.next().await {
            if let Ok(msg) = msg_result {
                let header = msg.header();
                if let Some(member) = header.member() {
                    if member.as_str() == "PropertiesChanged" {
                        if let Some(info) = get_battery_info().await {
                            let _ = app_handle.emit("battery-update", info);
                        }
                    }
                }
            }
        }
    });
}