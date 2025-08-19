mod app_url;
mod audio;
mod brightness;
mod commands;
mod eventloops;
mod menu_manager;
mod monitor_manager;
mod notifications;
mod structs;
mod tray;
mod window_manager;
mod windows_apps;

use commands::*;
use eventloops::{setup_windows_monitoring, setup_notification_monitoring};
use std::sync::{Arc, Mutex};
use structs::WMState;
use tauri_plugin_config_manager;
use tauri_plugin_user_data;
use tauri_plugin_network_manager;
use tauri_plugin_bluetooth_manager;
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
        .plugin(tauri_plugin_network_manager::init())
        .plugin(tauri_plugin_bluetooth_manager::init())
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
            toggle_menu,
            get_audio_volume,
            set_audio_volume,
            toggle_audio_mute,
            get_brightness_info,
            set_brightness_info,
            send_notify,
            clear_notifications,
            get_all_notifications,
            delete_notification,
            toggle_control_center,
        ])
        .setup(move |app| {
            let _ = create_desktops(app);
            let _ = create_panel(app);

            setup_windows_monitoring(window_manager.clone(), app.handle().clone())?;
            setup_notification_monitoring(app.handle().clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
