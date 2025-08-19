use crate::notifications::{initialize_app_handle, start_notification_monitor};
use crate::window_manager;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use window_manager::WindowManager;

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

pub fn setup_notification_monitoring(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        initialize_app_handle(app_handle).await;

        if let Err(e) = start_notification_monitor().await {
            eprintln!("Error starting notification monitor: {}", e);
        }
    });
}
