use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Monitor, WebviewUrl, WebviewWindowBuilder};

static AVAILABLE_MONITORS: OnceLock<Mutex<Option<Vec<Monitor>>>> = OnceLock::new();

pub fn get_monitors(app: &AppHandle) -> Option<Vec<Monitor>> {
    if AVAILABLE_MONITORS.get().is_none() {
        let temp_window =
            WebviewWindowBuilder::new(app, "temp", WebviewUrl::App("index.html".into()))
                .title("Temp")
                .inner_size(1.0, 1.0)
                .visible(false)
                .build()
                .ok()?;

        let monitors = temp_window.available_monitors().ok()?;
        AVAILABLE_MONITORS
            .set(Mutex::new(Some(monitors)))
            .expect("Failed to set AVAILABLE_MONITORS");
        // Cerrar la ventana temporal
        let _ = temp_window.close();
    }

    let monitors = AVAILABLE_MONITORS.get_or_init(|| Mutex::new(None));
    monitors.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

pub fn get_primary_monitor(app: &AppHandle) -> Option<Monitor> {
    if let Some(monitors) = get_monitors(app) {
        monitors
            .iter()
            .find(|monitor| monitor.position().x == 0 && monitor.position().y == 0)
            .or_else(|| monitors.first())
            .cloned()
    } else {
        None
    }
}
