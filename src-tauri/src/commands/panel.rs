use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn show_panel(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("panel") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }

    Ok(())
}