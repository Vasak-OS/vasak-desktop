use crate::structs::BatteryInfo;
use crate::applets::battery::{has_battery, get_battery_info};
use crate::logger::{log_debug, log_info};

#[tauri::command]
pub async fn battery_exists() -> bool {
    log_debug("Verificando existencia de batería");
    let has_bat = has_battery().await;
    log_debug(&format!("Batería presente: {}", has_bat));
    has_bat
}

#[tauri::command]
pub async fn battery_fetch_info() -> Option<BatteryInfo> {
    log_debug("Obteniendo información de batería");
    let info = get_battery_info().await;
    if let Some(ref battery) = info {
        log_info(&format!("Batería: {}% - {} - Cargando: {}", 
            battery.percentage, battery.state, battery.is_charging));
    } else {
        log_debug("No se pudo obtener información de batería");
    }
    info
}