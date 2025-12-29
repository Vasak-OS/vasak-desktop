mod app_url;
mod applets;
mod audio;
mod brightness;
mod commands;
mod dbus_service;
mod eventloops;
mod menu_manager;
mod monitor_manager;
mod notifications;
mod structs;
mod tray;
mod utils;
mod window_manager;
mod windows_apps;

use commands::*;
use eventloops::{
    setup_dbus_service,
    setup_windows_monitoring,
};
use std::sync::{Arc, Mutex};
use structs::WMState;
use tauri_plugin_bluetooth_manager;
use tauri_plugin_config_manager;
use tauri_plugin_network_manager;
use tauri_plugin_user_data;
use tray::create_tray_manager;
use window_manager::WindowManager;
use windows_apps::*;

use applets::{
    manager::AppletManager, 
    audio::AudioApplet,
    battery::BatteryApplet,
    bluetooth::BluetoothApplet,
    brightness::BrightnessApplet,
    music::MusicApplet,
    network::NetworkApplet,
    notifications::NotificationApplet, 
    tray::TrayApplet
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
        .manage(commands::ShortcutsState::new())
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
            open_configuration_window,
            logout,
            shutdown,
            reboot,
            suspend,
            detect_display_server,
            get_menu_items,
            toggle_menu,
            toggle_config_app,
            open_configuration_window,
            get_audio_volume,
            set_audio_volume,
            toggle_audio_mute,
            get_audio_devices,
            set_audio_device,
            toggle_audio_applet,
            get_brightness_info,
            set_brightness_info,
            toggle_system_theme,
            send_notify,
            clear_notifications,
            get_all_notifications,
            delete_notification,
            invoke_notification_action,
            toggle_control_center,
            toggle_network_applet,
            init_sni_watcher,
            get_tray_items,
            toggle_bluetooth_applet,
            music_play_pause,
            music_next_track,
            music_previous_track,
            music_now_playing,
            battery_exists,
            battery_fetch_info,
            global_search,
            execute_search_result,
            toggle_search,
            get_shortcuts,
            update_shortcut,
            add_custom_shortcut,
            delete_shortcut,
            execute_shortcut,
            check_shortcut_conflicts,
            get_system_info,
            get_cpu_usage_only,
            get_memory_usage_only,
            get_system_config,
            get_current_system_state,
            set_system_config,
            get_gtk_themes,
            get_cursor_themes,
            get_icon_packs,
            open_configuration_window
        ])
        .setup(move |app| {
            let _ = create_desktops(app);
            let _ = create_panel(app);

            setup_windows_monitoring(window_manager.clone(), app.handle().clone())?;
            setup_dbus_service(app.handle().clone());
            
            // Initialize global shortcuts handler
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let shortcuts_handler = utils::shortcuts::shortcuts_handler::GlobalShortcutsHandler::new();
                if let Err(e) = shortcuts_handler.register_all(app_handle).await {
                    log::warn!("Failed to register global shortcuts: {}", e);
                }
            });
            
            // Initialize AppletManager
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let manager = AppletManager::new();
                manager.register(AudioApplet).await;
                manager.register(BatteryApplet).await;
                manager.register(BluetoothApplet).await;
                manager.register(BrightnessApplet).await;
                manager.register(MusicApplet).await;
                manager.register(NetworkApplet).await;
                manager.register(TrayApplet).await;
                manager.register(NotificationApplet).await;
                
                manager.start_all(app_handle).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
