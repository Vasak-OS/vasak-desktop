use gtk::prelude::*;
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

use crate::monitor_manager::get_monitors;

pub fn create_desktops(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let monitors = get_monitors(app).ok_or("No monitors found")?;

    for (index, monitor) in monitors.iter().enumerate() {
        let monitor_size = monitor.size();
        let monitor_position = monitor.position();

        let desktop_window = WebviewWindowBuilder::new(
            app,
            &format!("desktop_{}", index),
            WebviewUrl::App(format!("index.html#/desktop?monitor={}", index).into()),
        )
        .title(&format!("Vasak Desktop {}", index))
        .decorations(false)
        .position(monitor_position.x as f64, monitor_position.y as f64)
        .inner_size(monitor_size.width as f64, monitor_size.height as f64)
        .max_inner_size(monitor_size.width as f64, monitor_size.height as f64)
        .min_inner_size(monitor_size.width as f64, monitor_size.height as f64)
        .skip_taskbar(true)
        .build()?;

        set_window_properties(&desktop_window);
    }

    Ok(())
}

fn set_window_properties(window: &tauri::WebviewWindow) {
    let gtk_window = window.gtk_window().expect("Failed to get GTK window");

    gtk_window.set_type_hint(gdk::WindowTypeHint::Desktop);
    gtk_window.set_skip_taskbar_hint(true);
}
