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
mod connect;
mod dbus_service;
mod eventloops;
/// Where the translations live.
///
/// The i18n plugin resolves them at runtime and only probes paths relative to
/// the executable and the working directory, none of which exist once the
/// binary is installed in /usr/bin — a packaged build would render raw keys.
fn locales_dir() -> Option<String> {
    let candidates = [
        std::path::PathBuf::from("locales"),
        std::path::PathBuf::from("src-tauri/locales"),
        std::path::PathBuf::from("/usr/share/vasak-desktop/locales"),
    ];

    candidates
        .into_iter()
        .find(|path| path.is_dir())
        .map(|path| path.to_string_lossy().into_owned())
}

/// Startup language from the session locale, falling back to Spanish, which is
/// what the shell shipped with before it was translatable.
fn default_locale() -> String {
    let raw = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();

    match raw.split(['_', '.', '@']).next().unwrap_or("") {
        "en" => "en".to_string(),
        _ => "es".to_string(),
    }
}

mod menu_manager;
mod menu_watcher;
mod monitor_manager;
mod notifications;
mod tray;
mod utils;
mod gtk_utils;
mod window_manager;
mod windows_apps;

use tauri::{Listener, Manager};
use commands::*;
use connect::{
    connect_launch_app, connect_list_apps, connect_list_devices, connect_list_running,
    connect_stop_app,
};
use dbus_pool::DbusPool;
use eventloops::{
    setup_dbus_service,
    setup_windows_monitoring,
};
use std::sync::{Arc, RwLock};
use structs::SystrayPopupState;
use structs::WMState;
use tokio::sync::watch;
use tray::create_tray_manager;
use window_manager::WindowManager;
use monitor_manager::watch_monitor_changes;
use windows_apps::*;

/// Shared latch signaled by the frontend when the panel has painted.
/// Registered *before* `create_panels` so no events are missed.
pub(crate) struct PanelReadyLatch(pub(crate) watch::Sender<bool>);

use applets::{
    manager::{AppletManager, AppletPriority},
    audio::AudioApplet,
    battery::BatteryApplet,
    bluetooth::BluetoothApplet,
    brightness::BrightnessApplet,
    connect::ConnectApplet,
    keyboard_leds::KeyboardLedsApplet,
    music::MusicApplet,
    network::NetworkApplet,
    network_rate::NetworkRateApplet,
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

    let cached_windows = Arc::new(parking_lot::RwLock::new(None));

    let wm_state = WMState {
        window_manager: window_manager.clone(),
        cached_windows: Arc::clone(&cached_windows),
    };

    let tray_manager = create_tray_manager();

    tauri::Builder::default()
        .manage(wm_state)
        .manage(tray_manager)
        .manage(SystrayPopupState(std::sync::Mutex::new(None)))
        .manage(WeatherCache::default())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_user_data::init())
        .plugin(tauri_plugin_network_manager::init())
        .plugin(tauri_plugin_bluetooth_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_vicons::init())
        .plugin(tauri_plugin_i18n_vsk::init_with_path(
            Some(default_locale()),
            locales_dir(),
        ))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            batch_invoke,
            weather_cached,
            weather_claim,
            weather_place,
            weather_release,
            weather_store,
            get_windows,
            toggle_window,
            open_app,
            open_settings,
            open_settings_section,
            open_panel_menu,
            show_osd,
            toggle_session_popup,
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
            hide_control_center,
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
            get_last_log_lines,
            connect_list_devices,
            connect_list_apps,
            connect_launch_app,
            connect_stop_app,
            connect_list_running,
            toggle_connect_menu
        ])
        .setup(move |app| {
            let setup_start = std::time::Instant::now();
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

            // Initialize shared D-Bus connection pool before applets.
            // Each bus (session/system) is tried independently; failures are logged
            // but stored as None so the pool is always available.
            let dbus_pool = tauri::async_runtime::block_on(DbusPool::init());
            app.manage(dbus_pool);

            // Register panel-ready listener BEFORE creating the panel, so the
            // readiness signal is available even if the frontend emits before
            // the deferred-applet task registers its own listener.
            let (ready_tx, _) = watch::channel(false);
            let ready_tx_clone = ready_tx.clone();
            app.listen("panel-ready", move |_| {
                let _ = ready_tx_clone.send(true);
            });
            app.manage(PanelReadyLatch(ready_tx));

            let handle = app.handle().clone();
            let _ = create_desktops(&handle);
            let _ = create_panels(&handle);
            // Built here, on the GTK main thread, and hidden until asked for.
            // Creating it later from an async task meant touching GTK from a
            // Tokio worker, which is not thread-safe and took the whole process
            // down the first time the button was pressed.
            if let Err(error) = create_control_center_window(&handle) {
                crate::logger::log_error(&format!("[control_center] no se pudo crear: {error}"));
            }
            watch_monitor_changes(&handle);
            menu_watcher::watch_application_dirs(&handle);

            // Retry in the background instead of aborting startup.
            //
            // This used to propagate with `?`, so if the Wayfire IPC socket was
            // not answering within its five second connect deadline — starting
            // a moment too early, or Wayfire still coming up — the entire shell
            // failed to launch. A compositor that is slow to appear should cost
            // a late taskbar, not the whole desktop.
            {
                let wm = window_manager.clone();
                let handle = app.handle().clone();
                let cached = cached_windows.clone();

                if let Err(error) = setup_windows_monitoring(wm.clone(), handle.clone(), cached.clone()) {
                    log::warn!("Wayfire IPC no disponible todavía ({error}); reintentando en segundo plano");

                    std::thread::spawn(move || {
                        let mut delay = std::time::Duration::from_secs(2);

                        loop {
                            std::thread::sleep(delay);

                            match setup_windows_monitoring(wm.clone(), handle.clone(), cached.clone()) {
                                Ok(()) => {
                                    log::info!("Wayfire IPC conectado; monitoreo de ventanas activo");
                                    break;
                                }
                                Err(error) => {
                                    log::debug!("Wayfire IPC sigue sin responder: {error}");
                                    delay = (delay * 2).min(std::time::Duration::from_secs(30));
                                }
                            }
                        }
                    });
                }
            }
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
                manager.register(KeyboardLedsApplet, AppletPriority::Normal).await;
                manager.register(MusicApplet, AppletPriority::Normal).await;
                manager.register(TrayApplet, AppletPriority::Normal).await;
                manager.register(NotificationApplet, AppletPriority::Normal).await;

                // Deferred: Started after panel-ready event from frontend
                manager.register(BluetoothApplet, AppletPriority::Deferred).await;
                manager.register(NetworkApplet, AppletPriority::Deferred).await;
                manager.register(NetworkRateApplet, AppletPriority::Deferred).await;
                // The phone service: nothing on screen depends on it, and most
                // sessions never plug one in.
                manager.register(ConnectApplet, AppletPriority::Deferred).await;
                
                manager.start_phased(app_handle).await;
                logger::log_info("Todos los applets iniciados correctamente");
            });

            logger::log_info(&format!("Setup callback completed in {:?}", setup_start.elapsed()));
            logger::log_info("Aplicación Tauri configurada correctamente");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
