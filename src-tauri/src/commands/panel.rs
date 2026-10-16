use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn show_panel(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("panel").ok_or_else(|| "panel window not found".to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}