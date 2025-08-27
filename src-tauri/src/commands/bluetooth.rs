use tauri::{async_runtime::spawn, AppHandle, Manager};
use crate::windows_apps::create_applet_bluetooth_window;

#[tauri::command]
pub fn toggle_bluetooth_applet(app: AppHandle) -> Result<(), ()> {
    if let Some(bluetooth_window) = app.get_webview_window("applet_bluetooth") {
        if bluetooth_window.is_visible().unwrap_or(false) {
            bluetooth_window.close().expect("Failed to close bluetooth window");
        } else {
            let _ = bluetooth_window.show();
            let _ = bluetooth_window.set_focus();
        }
    } else {
        spawn(async move {
            let _ = create_applet_bluetooth_window(app).await;
        });
    }

    Ok(())
}
