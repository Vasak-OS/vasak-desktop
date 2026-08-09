use crate::windows_apps::create_session_popup_window;
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub async fn toggle_session_popup(action: String, app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("session_popup") {
        if window.is_visible().unwrap_or(false) {
            window.hide().map_err(|e| e.to_string())?;
        } else {
            // The action is baked into the URL when the window is created, so
            // reusing a hidden window would show whatever was asked for last
            // time — click Reboot then Shutdown and you'd get the reboot
            // dialog. Send the new action before showing it.
            let _ = window.emit("session-action", action.clone());
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
