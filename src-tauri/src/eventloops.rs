use crate::window_manager;

use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use window_manager::WindowManager;

// Configuración del monitoreo de eventos
pub fn setup_event_monitoring(
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
