use tauri::{async_runtime::spawn, AppHandle, Manager};
use crate::windows_apps::create_app_configuration_window;

#[tauri::command]
pub fn toggle_config_app(app: AppHandle) -> Result<(), ()> {
    if let Some(network_window) = app.get_webview_window("app_config") {
        if network_window.is_visible().unwrap_or(false) {
            network_window.close().expect("Failed to close network window");
        } else {
            let _ = network_window.show();
            let _ = network_window.set_focus();
        }
    } else {
        spawn(async move {
            let _ = create_app_configuration_window(app).await;
        });
    }

    Ok(())
}
