use crate::brightness::{get_brightness, set_brightness};
use crate::logger::{log_info, log_error, log_debug};
use crate::structs::BrightnessInfo;

// Async for the same reason as the audio commands: these shell out to
// brightnessctl, and doing that on the main thread stalls the whole shell.
#[tauri::command]
pub async fn get_brightness_info() -> Result<BrightnessInfo, String> {
    log_debug("Comando: get_brightness_info");
    get_brightness().map_err(|e| {
        log_error(&format!("Error al obtener información de brillo: {}", e));
        e.to_string()
    })
}

#[tauri::command]
pub async fn set_brightness_info(brightness: u32) -> Result<(), String> {
    log_info(&format!("Estableciendo brillo a: {}%", brightness));
    set_brightness(brightness).map_err(|e| {
        log_error(&format!("Error al establecer brillo: {}", e));
        e.to_string()
    })
}
