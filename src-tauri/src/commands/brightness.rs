use tauri::AppHandle;

use crate::brightness::{get_brightness, set_brightness};
use crate::logger::{log_debug, log_error, log_info};
use crate::structs::BrightnessInfo;

/// El brillo del panel interno, leído de sysfs por el plugin de pantalla.
#[tauri::command]
pub fn get_brightness_info(app: AppHandle) -> Result<BrightnessInfo, String> {
    log_debug("Comando: get_brightness_info");
    get_brightness(&app).map_err(|e| {
        log_error(&format!("Error al obtener información de brillo: {}", e));
        e.to_string()
    })
}

/// Pone el brillo del panel por logind (D-Bus), sin subprocesos.
#[tauri::command]
pub async fn set_brightness_info(app: AppHandle, brightness: u32) -> Result<(), String> {
    log_info(&format!("Estableciendo brillo a: {}%", brightness));
    set_brightness(&app, brightness)
        .await
        .map_err(|e| e.to_string())
}
