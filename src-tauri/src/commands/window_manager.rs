use crate::window_manager::WindowInfo;
use crate::structs::WMState;
use crate::logger::{log_info, log_error, log_debug};
use std::time::Duration;

/// Maximum age for cached window state to be considered valid on lock timeout.
const MAX_CACHE_AGE: Duration = Duration::from_secs(5);
/// Lock acquisition timeout for IPC command handlers.
const LOCK_TIMEOUT: Duration = Duration::from_millis(50);

// Comandos de la API
#[tauri::command]
pub async fn get_windows(state: tauri::State<'_, WMState>) -> Result<Vec<WindowInfo>, String> {
    // Try to acquire the main RwLock with a 50ms timeout (Requirement 13.4)
    // Using try_read() first for non-blocking attempt
    match state.window_manager.try_read() {
        Ok(wm) => {
            match wm.get_window_list() {
                Ok(windows) => Ok(windows),
                Err(e) => {
                    log_error(&format!("Error al obtener lista de ventanas: {}", e));
                    // On IPC failure, try cached state
                    get_cached_or_empty(&state)
                }
            }
        }
        Err(_) => {
            // Lock is contended — use cached state with parking_lot timeout (Requirement 13.4, 13.5)
            log_debug("get_windows: lock contended, using cached state");
            get_cached_or_empty(&state)
        }
    }
}

/// Returns cached window list if fresh (<5s old), otherwise empty list.
/// Uses parking_lot::RwLock with try_read_for for timeout support.
fn get_cached_or_empty(state: &WMState) -> Result<Vec<WindowInfo>, String> {
    if let Some(guard) = state.cached_windows.try_read_for(LOCK_TIMEOUT) {
        if let Some(ref cached) = *guard {
            if cached.updated_at.elapsed() < MAX_CACHE_AGE {
                return Ok(cached.windows.clone());
            }
        }
    }
    // No valid cache or cache lock timed out — return empty list (Requirement 13.5)
    Ok(Vec::new())
}

#[tauri::command]
pub async fn toggle_window(window_id: String, state: tauri::State<'_, WMState>) -> Result<(), String> {
    log_info(&format!("Alternando ventana: {}", window_id));
    state
        .window_manager
        .read()
        .map_err(|e| {
            log_error(&format!("Error al bloquear window_manager: {}", e));
            e.to_string()
        })?
        .toggle_window(&window_id)
        .map_err(|e| {
            log_error(&format!("Error al alternar ventana {}: {}", window_id, e));
            e.to_string()
        })
}
