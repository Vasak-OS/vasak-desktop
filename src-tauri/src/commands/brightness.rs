use crate::brightness::{get_brightness, set_brightness, BrightnessInfo};

#[tauri::command]
pub fn get_brightness_info() -> Result<BrightnessInfo, String> {
    get_brightness()
}

#[tauri::command]
pub fn set_brightness_info(brightness: u32) -> Result<(), String> {
    set_brightness(brightness)
}
