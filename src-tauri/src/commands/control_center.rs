use tauri::{async_runtime::spawn, AppHandle, Manager};

use crate::logger::{log_info, log_warning};
use crate::windows_apps::create_control_center_window;


#[tauri::command]
pub fn toggle_control_center(app: AppHandle) -> Result<(), ()> {
    if let Some(control_center_window) = app.get_webview_window("control_center") {
        log_info("[control_center] toggle requested for existing window");
        if control_center_window.is_visible().unwrap_or(false) {
            log_info("[control_center] closing visible control center window");
            control_center_window.close().expect("Failed to close control center window");
        } else {
            log_info("[control_center] showing hidden control center window");
            let _ = control_center_window.show();
            let _ = control_center_window.set_focus();
        }
    } else {
        log_warning("[control_center] window not found, creating a new one");
        spawn(async move {
            let _ = create_control_center_window(app).await;
        });
    }

    Ok(())
}