use crate::window_manager;
use crate::window_manager::WindowInfo;
use crate::logger::{log_info, log_error};
use window_manager::WindowManager;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Emitter;

pub fn setup_windows_monitoring(
    window_manager: Arc<Mutex<WindowManager>>,
    app_handle: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    log_info("Configurando monitoreo de ventanas");
    let (tx, rx) = channel();

    {
        let mut wm = window_manager.lock().unwrap_or_else(|error| error.into_inner());
        wm.backend.setup_event_monitoring(tx)?;
        log_info("Monitoreo de eventos de ventanas establecido");
    }

    let event_handle = app_handle.clone();
    std::thread::spawn(move || {
        for _ in rx {
            let _ = event_handle.emit("window-update", ());
        }
    });

    let polling_manager = Arc::clone(&window_manager);
    let polling_handle = app_handle.clone();

    std::thread::spawn(move || {
        let mut last_snapshot: Option<Vec<WindowInfo>> = None;

        loop {
            let windows = {
                let mut wm = polling_manager.lock().unwrap_or_else(|error| error.into_inner());

                match wm.get_window_list() {
                    Ok(windows) => Some(windows),
                    Err(error) => {
                        log_error(&format!("Error obteniendo snapshot de ventanas: {}", error));
                        None
                    }
                }
            };

            if let Some(windows) = windows {
                if last_snapshot.as_ref() != Some(&windows) {
                    last_snapshot = Some(windows);
                    let _ = polling_handle.emit("window-update", ());
                }
            }

            std::thread::sleep(Duration::from_millis(1000));
        }
    });

    Ok(())
}

// Battery, Music, Notifications moved to AppletManager/Applets

pub fn setup_dbus_service(app_handle: tauri::AppHandle) {
    log_info("Iniciando servicio D-Bus");
    tauri::async_runtime::spawn(async move {
        if let Err(e) = crate::dbus_service::start_dbus_service(app_handle).await {
            log_error(&format!("Error al iniciar servicio D-Bus: {}", e));
            eprintln!("Error starting D-Bus service: {}", e);
        }
    });
}