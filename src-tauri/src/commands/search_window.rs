use crate::windows_apps::create_search_window;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn toggle_search(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("global_search") {
        if window.is_visible().unwrap_or(false) {
            window.hide().map_err(|e| e.to_string())?;
        } else {
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
        }
    } else {
        create_search_window(app)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
