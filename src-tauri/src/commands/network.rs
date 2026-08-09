use tauri::{async_runtime::spawn, AppHandle, Manager, Emitter};
use crate::windows_apps::create_applet_network_window;
use crate::logger::log_info;

#[tauri::command]
pub fn toggle_network_applet(app: AppHandle) -> Result<(), ()> {
    if let Some(network_window) = app.get_webview_window("applet_network") {
        if network_window.is_visible().unwrap_or(false) {
            log_info("Cerrando applet de red");
            // hide(), not close(): closing destroys the webview so the next open
            // reloads the page and re-runs Vue. The view refreshes on
            // "window-shown" instead.
            let _ = network_window.hide();
        } else {
            log_info("Mostrando applet de red");
            let _ = network_window.emit("window-shown", ());
            let _ = network_window.show();
            let _ = network_window.set_focus();
        }
    } else {
        log_info("Creando applet de red");
        spawn(async move {
            let _ = create_applet_network_window(app).await;
        });
    }

    Ok(())
}
