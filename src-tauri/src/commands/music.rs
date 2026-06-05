use crate::applets::music::{emit_now_playing, fetch_now_playing, mpris_next, mpris_playpause, mpris_previous};
use crate::logger::{log_info, log_error, log_debug};
use tauri::AppHandle;

#[tauri::command]
pub async fn music_play_pause(app: AppHandle, player: String) {
    log_info(&format!("Music: Play/Pause en reproductor: {}", player));
    match mpris_playpause(player.clone()).await {
        Ok(target) => {
            log_debug(&format!("Play/Pause ejecutado en {}", target));
            let _ = emit_now_playing(&app, &target).await;
        }
        Err(e) => {
            log_error(&format!("Error en play/pause de '{}': {}", player, e));
        }
    }
}

#[tauri::command]
pub async fn music_next_track(app: AppHandle, player: String) {
    log_info(&format!("Music: Siguiente track en reproductor: {}", player));
    match mpris_next(player.clone()).await {
        Ok(target) => {
            log_debug(&format!("Siguiente track ejecutado en {}", target));
            let _ = emit_now_playing(&app, &target).await;
        }
        Err(e) => {
            log_error(&format!("Error en siguiente track de '{}': {}", player, e));
        }
    }
}

#[tauri::command]
pub async fn music_previous_track(app: AppHandle, player: String) {
    log_info(&format!("Music: Track anterior en reproductor: {}", player));
    match mpris_previous(player.clone()).await {
        Ok(target) => {
            log_debug(&format!("Track anterior ejecutado en {}", target));
            let _ = emit_now_playing(&app, &target).await;
        }
        Err(e) => {
            log_error(&format!("Error en track anterior de '{}': {}", player, e));
        }
    }
}

#[tauri::command]
pub async fn music_now_playing() -> Result<serde_json::Value, String> {
    log_debug("Obteniendo información de reproducción actual");
    let result = fetch_now_playing().await;
    if let Err(ref e) = result {
        log_error(&format!("Error obteniendo información de reproducción: {}", e));
    }
    result
}