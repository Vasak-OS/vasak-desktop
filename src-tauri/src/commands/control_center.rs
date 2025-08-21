use tauri::{async_runtime::spawn, AppHandle, Manager};
use crate::windows_apps::create_control_center_window;


#[tauri::command]
pub fn toggle_control_center(app: AppHandle) -> Result<(), ()> {
    if let Some(control_center_window) = app.get_webview_window("control_center") {
        if control_center_window.is_visible().unwrap_or(false) {
            control_center_window.close().expect("Failed to close control center window");
        } else {
            let _ = control_center_window.show();
            let _ = control_center_window.set_focus();
        }
    } else {
        spawn(async move {
            let _ = create_control_center_window(app).await;
        });
    }

    Ok(())
}