use gtk::prelude::*;
use tauri::{WebviewUrl, WebviewWindowBuilder, AppHandle};

use crate::monitor_manager::get_primary_monitor;

pub fn create_panel(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(app)
        .ok_or("No primary monitor found")?;
    let primary_monitor_size = primary_monitor.size();
    let primary_monitor_position = primary_monitor.position();

    // Crear la ventana del panel solo en el monitor primario
    let panel_window =
        WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("index.html#/panel".into()))
            .title("Vasak Panel")
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .position(
                primary_monitor_position.x as f64,
                primary_monitor_position.y as f64,
            )
            .inner_size(primary_monitor_size.width as f64, 32.0)
            .max_inner_size(primary_monitor_size.width as f64, 32.0)
            .min_inner_size(primary_monitor_size.width as f64, 32.0)
            .build()?;

    set_window_properties(&panel_window);
    Ok(())
}

pub fn set_window_properties(window: &tauri::WebviewWindow) {
    let gtk_window = window.gtk_window().expect("Failed to get GTK window");

    gtk_window.set_type_hint(gdk::WindowTypeHint::Dock);
    gtk_window.set_skip_taskbar_hint(true);
}
