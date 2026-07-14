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

/// Maximum time to wait for the window_manager read lock before returning an error.
const TOGGLE_LOCK_TIMEOUT: Duration = Duration::from_millis(100);
/// Retry interval when the read lock is contended.
const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(10);

#[tauri::command]
pub async fn toggle_window(window_id: String, state: tauri::State<'_, WMState>) -> Result<(), String> {
    log_info(&format!("Alternando ventana: {}", window_id));

    // Try non-blocking read first (Requirement 13.4, 13.6)
    if let Ok(wm) = state.window_manager.try_read() {
        return wm.toggle_window(&window_id).map_err(|e| {
            log_error(&format!("Error al alternar ventana {}: {}", window_id, e));
            e.to_string()
        });
    }

    // Lock is contended — retry with short sleeps up to 100ms (Requirement 13.6)
    log_debug("toggle_window: lock contended, retrying with timeout");
    let start = std::time::Instant::now();
    loop {
        std::thread::sleep(LOCK_RETRY_INTERVAL);

        if let Ok(wm) = state.window_manager.try_read() {
            return wm.toggle_window(&window_id).map_err(|e| {
                log_error(&format!("Error al alternar ventana {}: {}", window_id, e));
                e.to_string()
            });
        }

        if start.elapsed() >= TOGGLE_LOCK_TIMEOUT {
            log_error(&format!(
                "toggle_window: lock timeout after {:?} for window {}",
                TOGGLE_LOCK_TIMEOUT, window_id
            ));
            return Err(format!(
                "Window manager busy, could not toggle window {} within {:?}",
                window_id, TOGGLE_LOCK_TIMEOUT
            ));
        }
    }
}
