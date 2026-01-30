use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Monitor, WebviewUrl, WebviewWindowBuilder};
use crate::logger::{log_info, log_debug, log_error};

static AVAILABLE_MONITORS: OnceLock<Mutex<Option<Vec<Monitor>>>> = OnceLock::new();

pub fn get_monitors(app: &AppHandle) -> Option<Vec<Monitor>> {
    if AVAILABLE_MONITORS.get().is_none() {
        log_debug("Detectando monitores disponibles");
        let temp_window =
            WebviewWindowBuilder::new(app, "temp", WebviewUrl::App("index.html".into()))
                .title("Temp")
                .inner_size(1.0, 1.0)
                .visible(false)
                .build()
                .ok()?;

        let monitors = temp_window.available_monitors().ok()?;
        log_info(&format!("Detectados {} monitores", monitors.len()));
        for (i, monitor) in monitors.iter().enumerate() {
            log_debug(&format!("  Monitor {}: {}x{} en ({},{})", 
                i, monitor.size().width, monitor.size().height,
                monitor.position().x, monitor.position().y));
        }
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
    log_debug("Obteniendo monitor primario");
    if let Some(monitors) = get_monitors(app) {
        let primary = monitors
            .iter()
            .find(|monitor| monitor.position().x == 0 && monitor.position().y == 0)
            .or_else(|| monitors.first())
            .cloned();
        if let Some(ref mon) = primary {
            log_debug(&format!("Monitor primario: {}x{}", 
                mon.size().width, mon.size().height));
        }
        primary
    } else {
        log_error("No se pudieron obtener monitores");
        None
    }
}
