mod app_url;
mod commands;
mod eventloops;
mod monitor_manager;
mod structs;
mod tray;
mod window_manager;
mod windows_apps;
mod menu_manager;

use commands::*;
use eventloops::setup_event_monitoring;
use std::sync::{Arc, Mutex};
use structs::WMState;
use tauri::Manager;
use tauri_plugin_config_manager;
use tauri_plugin_user_data;
use tray::create_tray_manager;
use window_manager::WindowManager;
use windows_apps::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize Windows Manager
    let window_manager = Arc::new(Mutex::new(
        WindowManager::new().expect("Failed to initialize window manager"),
    ));

    let wm_state = WMState {
        window_manager: window_manager.clone(),
    };

    let tray_manager = create_tray_manager();

    tauri::Builder::default()
        .manage(wm_state)
        .manage(tray_manager)
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_user_data::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("panel") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_vicons::init())
        .invoke_handler(tauri::generate_handler![
            get_windows,
            toggle_window,
            open_app,
            logout,
            shutdown,
            reboot,
            suspend,
            detect_display_server,
            get_menu_items,
            toggle_menu
        ])
        .setup(move |app| {
            let _ = create_desktops(app);
            let _ = create_panel(app);

            setup_event_monitoring(window_manager.clone(), app.handle().clone())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
