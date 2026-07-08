use crate::windows_apps::create_session_popup_window;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn toggle_session_popup(action: String, app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("session_popup") {
        if window.is_visible().unwrap_or(false) {
            window.close().map_err(|e| e.to_string())?;
        } else {
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
        }
    } else {
        create_session_popup_window(app, action)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
