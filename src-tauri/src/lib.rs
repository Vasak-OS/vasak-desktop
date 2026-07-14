// Core modules
mod app_url;
mod constants;
mod dbus_pool;
mod error;
mod logger;
mod structs;

// Feature modules
mod applets;
mod audio;
mod audio_native;
mod brightness;
mod commands;
mod dbus_service;
mod eventloops;
mod menu_manager;
mod monitor_manager;
mod notifications;
mod tray;
mod utils;
mod gtk_utils;
mod window_manager;
mod windows_apps;

use commands::*;
use dbus_pool::DbusPool;
use eventloops::{
    setup_dbus_service,
    setup_windows_monitoring,
};
use std::sync::{Arc, RwLock};
use structs::SystrayPopupState;
use structs::WMState;
use tray::create_tray_manager;
use window_manager::WindowManager;
use windows_apps::*;

use applets::{
    manager::{AppletManager, AppletPriority},
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
    // Inicializar el sistema de logging
    logger::log_info("Vasak Desktop iniciando...");
    
    let window_manager = Arc::new(RwLock::new(
        WindowManager::new().expect("Failed to initialize window manager"),
    ));

    let wm_state = WMState {
        window_manager: window_manager.clone(),
    };

    let tray_manager = create_tray_manager();

    tauri::Builder::default()
        .manage(wm_state)
        .manage(tray_manager)
        .manage(SystrayPopupState(std::sync::Mutex::new(None)))
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_user_data::init())
        .plugin(tauri_plugin_network_manager::init())
        .plugin(tauri_plugin_bluetooth_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_vicons::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            batch_invoke,
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
            show_panel,
            get_audio_volume,
            set_audio_volume,
            toggle_audio_mute,
            get_audio_devices,
            set_audio_device,
            toggle_audio_applet,
            get_brightness_info,
            set_brightness_info,
            send_notify,
            clear_notifications,
            get_all_notifications,
            delete_notification,
            invoke_notification_action,
            toggle_control_center,
            toggle_network_applet,
            init_sni_watcher,
            get_tray_items,
            tray_item_activate,
            tray_item_secondary_activate,
            get_tray_menu,
            tray_menu_item_click,
            open_tray_popup,
            get_tray_popup_data,
            tray_popup_click,
            toggle_bluetooth_applet,
            music_play_pause,
            music_next_track,
            music_previous_track,
            music_now_playing,
            battery_exists,
            battery_fetch_info,
            get_battery_info,
            global_search,
            execute_search_result,
            toggle_search,
            log_from_frontend,
            get_log_file_path,
            read_log_file,
            get_last_log_lines
        ])
        .setup(move |app| {
            logger::log_info("Configurando aplicación Tauri...");

            // Suprimir Gdk-CRITICAL de inicialización Wayland (internos de GDK,
            // inofensivos pero ruidosos).
            glib::log_set_handler(
                Some("Gdk"),
                glib::LogLevels::LEVEL_CRITICAL,
                false,  // fatal
                false,  // recursion
                |_domain, _level, _message| {},
            );

            // Initialize shared D-Bus connection pool before applets
            let dbus_pool = tauri::async_runtime::block_on(async {
                match DbusPool::init().await {
                    Ok(pool) => pool,
                    Err(e) => {
                        logger::log_info(&format!(
                            "DbusPool: error inicializando conexiones D-Bus: {}. Continuando sin pool.",
                            e
                        ));
                        // Create a pool with empty connections as fallback
                        DbusPool::init().await.unwrap_or_else(|_| {
                            // This path should rarely happen; if it does, applets
                            // will create their own connections as before.
                            panic!("No se pudo establecer conexión D-Bus después de reintentar");
                        })
                    }
                }
            });
            app.manage(dbus_pool);

            let _ = create_desktops(app);
            let _ = create_panel(app);

            setup_windows_monitoring(window_manager.clone(), app.handle().clone())?;
            setup_dbus_service(app.handle().clone());
            
            // Initialize AppletManager with priority-based phased startup
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let manager = Arc::new(AppletManager::new());

                // Critical: Audio and Brightness must be ready before others
                manager.register(AudioApplet, AppletPriority::Critical).await;
                manager.register(BrightnessApplet, AppletPriority::Critical).await;

                // Normal: Spawned after critical are ready, without awaiting
                manager.register(BatteryApplet, AppletPriority::Normal).await;
                manager.register(MusicApplet, AppletPriority::Normal).await;
                manager.register(TrayApplet, AppletPriority::Normal).await;
                manager.register(NotificationApplet, AppletPriority::Normal).await;

                // Deferred: Started after panel-ready event from frontend
                manager.register(BluetoothApplet, AppletPriority::Deferred).await;
                manager.register(NetworkApplet, AppletPriority::Deferred).await;
                
                manager.start_phased(app_handle).await;
                logger::log_info("Todos los applets iniciados correctamente");
            });

            logger::log_info("Aplicación Tauri configurada correctamente");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
