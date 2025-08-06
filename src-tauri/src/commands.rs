use crate::window_manager::WindowInfo;
use crate::structs::WMState; // Use crate::structs if the module exists outside this folder

// Comandos de la API
#[tauri::command]
pub async fn get_windows(state: tauri::State<'_, WMState>) -> Result<Vec<WindowInfo>, String> {
    state
        .window_manager
        .lock()
        .map_err(|e| e.to_string())?
        .get_window_list()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_window(window_id: String, state: tauri::State<'_, WMState>) -> Result<(), String> {
    state
        .window_manager
        .lock()
        .map_err(|e| e.to_string())?
        .toggle_window(&window_id)
        .map_err(|e| e.to_string())
}
