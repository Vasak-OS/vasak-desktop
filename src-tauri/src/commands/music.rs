use crate::applets::music::{emit_now_playing, fetch_now_playing, mpris_next, mpris_playpause, mpris_previous};
use tauri::AppHandle;

#[tauri::command]
pub fn music_play_pause(app: AppHandle, player: String) {
    if let Ok(target) = mpris_playpause(player) {
        let _ = emit_now_playing(&app, &target);
    }
}

#[tauri::command]
pub fn music_next_track(app: AppHandle, player: String) {
    if let Ok(target) = mpris_next(player) {
        let _ = emit_now_playing(&app, &target);
    }
}

#[tauri::command]
pub fn music_previous_track(app: AppHandle, player: String) {
    if let Ok(target) = mpris_previous(player) {
        let _ = emit_now_playing(&app, &target);
    }
}

#[tauri::command]
pub fn music_now_playing() -> Result<serde_json::Value, String> {
    fetch_now_playing()
}