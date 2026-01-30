use tauri::{async_runtime::spawn, AppHandle, Manager};
use crate::logger::{log_info, log_debug};
use crate::windows_apps::{create_app_configuration_window, create_file_manager_window};

#[tauri::command]
pub fn open_file_manager_window(app: AppHandle, path: Option<String>) -> Result<(), String> {
    log_info(&format!("Abriendo gestor de archivos: {:?}", path));
    spawn(async move {
        let _ = create_file_manager_window(app, path);
    });

    Ok(())
}


#[tauri::command]
pub fn toggle_config_app(app: AppHandle) -> Result<(), ()> {
    log_debug("Alternando ventana de configuración");
    if let Some(menu_window) = app.get_webview_window("menu") {
        let _ = menu_window.close();
    }

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

#[tauri::command]
pub fn open_configuration_window(app: AppHandle) -> Result<(), String> {
    log_info("Abriendo ventana de configuración");
    // Cerrar el menú si está abierto para evitar solapamiento visual
    if let Some(menu_window) = app.get_webview_window("menu") {
        let _ = menu_window.close();
    }

    if let Some(config_window) = app.get_webview_window("app_config") {
        let _ = config_window.show();
        let _ = config_window.set_focus();
    } else {
        spawn(async move {
            let _ = create_app_configuration_window(app).await;
        });
    }

    Ok(())
}

