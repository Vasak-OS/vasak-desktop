use crate::applets::music::{mpris_next, mpris_playpause, mpris_previous, fetch_now_playing};

#[tauri::command]
pub fn music_play_pause(player: String) {
    let _ = mpris_playpause(player);
}

#[tauri::command]
pub fn music_next_track(player: String) {
    let _ = mpris_next(player);
}

#[tauri::command]
pub fn music_previous_track(player: String) {
    let _ = mpris_previous(player);
}

#[tauri::command]
pub fn music_now_playing() -> Result<serde_json::Value, String> {
    fetch_now_playing()
}