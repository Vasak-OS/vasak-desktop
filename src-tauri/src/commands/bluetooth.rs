use tauri::{async_runtime::spawn, AppHandle, Manager};
use crate::windows_apps::create_applet_bluetooth_window;
use crate::logger::log_info;

#[tauri::command]
pub fn toggle_bluetooth_applet(app: AppHandle) -> Result<(), ()> {
    if let Some(bluetooth_window) = app.get_webview_window("applet_bluetooth") {
        if bluetooth_window.is_visible().unwrap_or(false) {
            log_info("Cerrando applet de Bluetooth");
            bluetooth_window.close().expect("Failed to close bluetooth window");
        } else {
            log_info("Mostrando applet de Bluetooth");
            let _ = bluetooth_window.show();
            let _ = bluetooth_window.set_focus();
        }
    } else {
        log_info("Creando applet de Bluetooth");
        spawn(async move {
            let _ = create_applet_bluetooth_window(app).await;
        });
    }

    Ok(())
}
