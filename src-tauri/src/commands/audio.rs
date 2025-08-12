use crate::audio::{get_volume, set_volume, toggle_mute};

#[tauri::command]
pub fn get_audio_volume() -> Result<VolumeInfo, String> {
    get_volume()
}

#[tauri::command]
pub fn set_audio_volume(volume: i64) -> Result<(), String> {
    set_volume(volume)
}

#[tauri::command]
pub fn toggle_audio_mute() -> Result<bool, String> {
    toggle_mute()
}
