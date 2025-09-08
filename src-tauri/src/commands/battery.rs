use crate::structs::BatteryInfo;
use crate::battery::{has_battery, get_battery_info};

#[tauri::command]
pub async fn battery_exists() -> bool {
    has_battery().await
}

#[tauri::command]
pub async fn battery_fetch_info() -> Option<BatteryInfo> {
    get_battery_info().await
}