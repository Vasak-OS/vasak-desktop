use crate::window_manager::WindowInfo;
use crate::structs::WMState;
use crate::logger::{log_info, log_error, log_debug};

// Comandos de la API
#[tauri::command]
pub async fn get_windows(state: tauri::State<'_, WMState>) -> Result<Vec<WindowInfo>, String> {
    log_debug("Obteniendo lista de ventanas");
    state
        .window_manager
        .lock()
        .map_err(|e| {
            log_error(&format!("Error al bloquear window_manager: {}", e));
            e.to_string()
        })?
        .get_window_list()
        .map_err(|e| {
            log_error(&format!("Error al obtener lista de ventanas: {}", e));
            e.to_string()
        })
}

#[tauri::command]
pub async fn toggle_window(window_id: String, state: tauri::State<'_, WMState>) -> Result<(), String> {
    log_info(&format!("Alternando ventana: {}", window_id));
    state
        .window_manager
        .lock()
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
