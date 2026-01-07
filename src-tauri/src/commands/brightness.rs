use crate::brightness::{get_brightness, set_brightness};
use crate::structs::BrightnessInfo;

#[tauri::command]
pub fn get_brightness_info() -> Result<BrightnessInfo, String> {
    get_brightness().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_brightness_info(brightness: u32) -> Result<(), String> {
    set_brightness(brightness).map_err(|e| e.to_string())
}
