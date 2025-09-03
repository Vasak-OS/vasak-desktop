use tauri::AppHandle;

use crate::audio::{get_volume, set_volume, toggle_mute};
use crate::structs::VolumeInfo;

#[tauri::command]
pub fn get_audio_volume() -> Result<VolumeInfo, String> {
    get_volume()
}

#[tauri::command]
pub fn set_audio_volume(volume: i64, app: AppHandle) -> Result<(), String> {
    set_volume(volume, app)
}

#[tauri::command]
pub fn toggle_audio_mute(app: AppHandle) -> Result<bool, String> {
    toggle_mute(app)
}
