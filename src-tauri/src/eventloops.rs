use crate::window_manager;
use window_manager::WindowManager;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use tauri::Emitter;

pub fn setup_windows_monitoring(
    window_manager: Arc<Mutex<WindowManager>>,
    app_handle: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = channel();

    if let Ok(mut wm) = window_manager.lock() {
        wm.backend.setup_event_monitoring(tx)?;
    }

    std::thread::spawn(move || {
        for _ in rx {
            let _ = app_handle.emit("window-update", ());
        }
    });

    Ok(())
}

// Battery, Music, Notifications moved to AppletManager/Applets

pub fn setup_dbus_service(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = crate::dbus_service::start_dbus_service(app_handle).await {
            eprintln!("Error starting D-Bus service: {}", e);
        }
    });
}