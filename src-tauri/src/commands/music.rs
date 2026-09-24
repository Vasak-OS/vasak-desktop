use crate::applets::music::{
    emit_now_playing, fetch_now_playing, list_players, mpris_next, mpris_playpause, mpris_previous,
    mpris_raise, mpris_set_loop, mpris_set_position, mpris_set_shuffle, mpris_set_volume,
    mpris_stop, pick_player,
};
use crate::artwork;
use crate::logger::{log_debug, log_error, log_info};
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
    log_info(&format!(
        "Music: Siguiente track en reproductor: {}",
        player
    ));
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
        log_error(&format!(
            "Error obteniendo información de reproducción: {}",
            e
        ));
    }
    result
}
#[tauri::command]
pub async fn music_stop(app: AppHandle, player: String) -> Result<(), String> {
    let target = mpris_stop(player).await.inspect_err(|e| {
        log_error(&format!("Error parando la reproducción: {}", e));
    })?;
    let _ = emit_now_playing(&app, &target).await;
    Ok(())
}

#[tauri::command]
pub async fn music_raise(player: String) -> Result<(), String> {
    mpris_raise(player).await.map(|_| ()).inspect_err(|e| {
        log_error(&format!("Error trayendo el reproductor al frente: {}", e));
    })
}

/// Salta a un punto de la pista. La posición va en microsegundos, como MPRIS.
#[tauri::command]
pub async fn music_set_position(
    app: AppHandle,
    player: String,
    position: i64,
) -> Result<(), String> {
    let target = mpris_set_position(player, position)
        .await
        .inspect_err(|e| {
            log_error(&format!("Error saltando a {}: {}", position, e));
        })?;
    let _ = emit_now_playing(&app, &target).await;
    Ok(())
}

#[tauri::command]
pub async fn music_set_volume(app: AppHandle, player: String, volume: f64) -> Result<(), String> {
    let target = mpris_set_volume(player, volume).await.inspect_err(|e| {
        log_error(&format!("Error poniendo el volumen: {}", e));
    })?;
    let _ = emit_now_playing(&app, &target).await;
    Ok(())
}

#[tauri::command]
pub async fn music_set_shuffle(
    app: AppHandle,
    player: String,
    shuffle: bool,
) -> Result<(), String> {
    let target = mpris_set_shuffle(player, shuffle).await.inspect_err(|e| {
        log_error(&format!("Error cambiando el aleatorio: {}", e));
    })?;
    let _ = emit_now_playing(&app, &target).await;
    Ok(())
}

#[tauri::command]
pub async fn music_set_loop(app: AppHandle, player: String, status: String) -> Result<(), String> {
    let target = mpris_set_loop(player, status).await.inspect_err(|e| {
        log_error(&format!("Error cambiando la repetición: {}", e));
    })?;
    let _ = emit_now_playing(&app, &target).await;
    Ok(())
}

/// Los reproductores que hay ahora mismo, para poder elegir entre ellos.
#[tauri::command]
pub async fn music_players() -> Result<serde_json::Value, String> {
    list_players().await.inspect_err(|e| {
        log_error(&format!("Error listando los reproductores: {}", e));
    })
}

/// Elige un reproductor a mano; con el vacío vuelve la elección automática.
#[tauri::command]
pub async fn music_select_player(app: AppHandle, player: String) -> Result<(), String> {
    log_info(&format!("Music: reproductor elegido a mano: {}", player));
    pick_player(&app, player)
        .await
        .map(|_| ())
        .inspect_err(|e| {
            log_error(&format!("Error eligiendo el reproductor: {}", e));
        })
}

/// Los bytes de la carátula, que el WebView no puede leer del disco por su
/// cuenta. Ver `crate::artwork` para por qué no los lee.
#[tauri::command]
pub async fn music_artwork(url: String) -> Result<tauri::ipc::Response, String> {
    let bytes = artwork::read_local(&url).await.inspect_err(|e| {
        log_debug(&format!("Carátula no disponible: {}", e));
    })?;
    Ok(tauri::ipc::Response::new(bytes))
}
