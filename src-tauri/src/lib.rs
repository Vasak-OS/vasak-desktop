// Core modules
mod app_url;
mod constants;
mod dbus_pool;
mod error;
mod listo;
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

mod desktop_watcher;
mod gtk_utils;
mod inotify_rafaga;
mod menu_manager;
mod menu_watcher;
mod monitor_manager;
mod notifications;
mod tray;
mod utils;
mod window_manager;
mod windows_apps;

use commands::*;
use connect::{
    connect_launch_app, connect_list_apps, connect_list_cameras, connect_list_devices,
    connect_list_running, connect_start_webcam, connect_stop_app, connect_stop_webcam,
    connect_webcam_state,
};
use dbus_pool::DbusPool;
use eventloops::{setup_dbus_service, setup_windows_monitoring};
use monitor_manager::watch_monitor_changes;
use std::sync::{Arc, RwLock};
use structs::SystrayPopupState;
use structs::WMState;
use tauri::{Listener, Manager};
use tokio::sync::watch;
use tray::create_tray_manager;
use window_manager::WindowManager;
use windows_apps::*;

/// Shared latch signaled by the frontend when the panel has painted.
/// Registered *before* `create_panels` so no events are missed.
pub(crate) struct PanelReadyLatch(pub(crate) watch::Sender<bool>);

use applets::{
    audio::AudioApplet,
    battery::BatteryApplet,
    bluetooth::BluetoothApplet,
    brightness::BrightnessApplet,
    connect::ConnectApplet,
    keyboard_leds::KeyboardLedsApplet,
    manager::{AppletManager, AppletPriority},
    music::MusicApplet,
    network::NetworkApplet,
    network_rate::NetworkRateApplet,
    notifications::NotificationApplet,
    privacidad::PrivacidadApplet,
    tray::TrayApplet,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Antes que nada: desde acá el registro se vuelca solo cada pocos segundos.
    //
    // Va primero porque lo que viene después —construir Tauri, arrancar los
    // plugins— es justo el tramo que no llegaba al archivo cuando el proceso
    // moría, y es el que se quiere leer cuando el escritorio no aparece.
    logger::volcar_cada_tanto();
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
        // El diario del sistema, con el nombre de esta aplicación. Va **primero**
        // de todos los plugins: instala el gancho de pánico, y un pánico mientras
        // arranca otro plugin es de los más probables y de los que menos rastro
        // dejan — sin esto sólo queda un volcado de núcleo sin símbolos.
        .plugin(tauri_plugin_vsk_journal::init())
        .plugin(tauri_plugin_vsk_contextual_menu::init())
        .invoke_handler(tauri::generate_handler![
            batch_invoke,
            privacidad_en_uso,
            privacidad_cortar,
            toggle_privacidad_applet,
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
            twingate_info,
            toggle_twingate_applet,
            twingate_authorize,
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
            connect_list_cameras,
            connect_start_webcam,
            connect_stop_webcam,
            connect_webcam_state,
            toggle_connect_menu
        ])
        .setup(move |app| {
            // El puente de `log` se instala **acá**, después de los plugins, y
            // no antes de construir Tauri.
            //
            // `tauri-plugin-bluetooth-manager` instala su propio
            // `tracing-subscriber` con `.init()`, que reclama el mismo slot
            // global de `log` y **paniquea** si ya está tomado: con el puente
            // primero, el escritorio no arrancaba —«failed to set global default
            // subscriber»— y eso llegó a main sin que nadie lo notara, porque
            // lanzar el shell para probarlo se lleva puesta la sesión.
            //
            // Instalándolo después gana el plugin y el puente queda inerte sin
            // romper nada.
            //
            // Ojo con lo que **no** arregla el `try_init` del plugin: ese cambio
            // sólo evita el panic, no cambia el orden. Mientras esta llamada
            // siga acá, el plugin inicializa primero y se queda con el slot
            // igual. Para que vuelva a ganar el puente —y los mensajes de `log`
            // terminen en el registro de la aplicación en lugar del del
            // plugin— hay que mover esta línea de vuelta antes de
            // `tauri::Builder::default()`, y eso sólo es seguro una vez que la
            // versión del plugin con `try_init` esté publicada.
            logger::install_log_bridge();

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

            // Y el mismo evento es el que le avisa a systemd. El escritorio es
            // una unidad `Type=notify` ordenada delante del inicio automático:
            // hasta que no diga que está listo, no arranca nada más. `panel-ready`
            // es la señal correcta porque la emite el panel **cuando pintó**, no
            // cuando se lo creó.
            //
            // Una sola vez: el evento puede repetirse si el panel se recrea —al
            // cambiar de monitores, por ejemplo— y avisar de nuevo no aporta
            // nada. `Ordering::SeqCst` y no `Relaxed` porque acá lo barato es la
            // garantía y lo caro sería depurar un aviso perdido.
            let ya_aviso = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let ya_aviso_panel = ya_aviso.clone();

            app.listen("panel-ready", move |_| {
                let _ = ready_tx_clone.send(true);
                if !ya_aviso_panel.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    listo::avisar_que_esta_listo();
                    logger::log_info("Listo: el panel pintó, se avisó a systemd");
                }
            });
            app.manage(PanelReadyLatch(ready_tx));

            // El respaldo. Un `Type=notify` que nunca avisa es una unidad que
            // systemd da por fallida y mata a los treinta segundos: sería
            // cambiar «el fondo tarda» por «no hay escritorio». Si el panel no
            // reportó en este plazo se avisa igual y queda anotado por qué.
            //
            // Veinte segundos: el arranque medido con el sistema tranquilo es de
            // 467 ms, así que llegar acá ya significa que algo anda mal. El tope
            // de la unidad está por encima para que este camino gane siempre.
            {
                let ya_aviso = ya_aviso.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                    if !ya_aviso.swap(true, std::sync::atomic::Ordering::SeqCst) {
                        listo::avisar_que_esta_listo();
                        logger::log_warning(
                            "Listo: el panel no reportó en 20 s; se avisa a systemd igual \
                             para no dejar la sesión sin abrir nada",
                        );
                    }
                });
            }

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
            // La carpeta del escritorio, para que el widget de archivos deje de
            // releerla cada diez segundos sin motivo.
            desktop_watcher::watch_desktop_dir(&handle);

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
                // Quién te mira y quién te escucha: nadie abre la cámara en el
                // primer segundo de sesión, y recorrer /proc no tiene por qué
                // competir con lo que dibuja el panel.
                manager.register(PrivacidadApplet, AppletPriority::Deferred).await;
                
                manager.start_phased(app_handle).await;
                logger::log_info("Todos los applets iniciados correctamente");
            });

            logger::log_info(&format!(
                "Setup callback completed in {:?}",
                setup_start.elapsed()
            ));
            logger::log_info("Aplicación Tauri configurada correctamente");

            // Y acá, a mano, además del volcado periódico: la traza del arranque
            // entera queda en el archivo apenas termina, sin esperar hasta cinco
            // segundos. Es una escritura por sesión.
            //
            // **Después de la última línea del setup, no antes.** Volcar antes
            // deja fuera justamente la que dice que el arranque terminó bien, que
            // es la que distingue «no llegó a configurarse» de «se configuró y
            // murió después».
            logger::flush();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
