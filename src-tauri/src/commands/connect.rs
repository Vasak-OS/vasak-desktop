use crate::windows_apps::create_connect_window;
use tauri::{async_runtime::spawn, AppHandle, Manager};

/// Shows or hides the phone's application list.
///
/// Same shape as `toggle_menu`, and for the same reason: `hide()` rather than
/// `close()`, so reopening does not destroy the webview and re-run Vue from
/// scratch. The list is kept current by the service's signals instead.
#[tauri::command]
pub fn toggle_connect_menu(app: AppHandle) -> Result<(), tauri::Error> {
    if let Some(window) = app.get_webview_window("connect") {
        if window.is_visible().unwrap_or(false) {
            window.hide()?;
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    } else {
        spawn(async move {
            let _ = create_connect_window(app).await;
        });
    }

    Ok(())
}
