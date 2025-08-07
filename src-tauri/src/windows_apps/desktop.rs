use gtk::prelude::*;
use tauri::{App, Manager};

use crate::monitor_manager::{get_monitors, get_primary_monitor};

pub fn create_desktops(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let _monitors = get_monitors(app.handle()).ok_or("No monitors found")?;
    let primary_monitor = get_primary_monitor(app.handle()).ok_or("No primary monitor found")?;

    let primary_monitor_size = primary_monitor.size();
    let primary_monitor_position = primary_monitor.position();

    let desktop_window = app
        .get_webview_window("desktop")
        .expect("desktop window not found");

    desktop_window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
        x: primary_monitor_position.x,
        y: primary_monitor_position.y,
    }))?;

    desktop_window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
        width: primary_monitor_size.width,
        height: primary_monitor_size.height,
    }))?;

    desktop_window.set_max_size(Some(tauri::Size::Physical(tauri::PhysicalSize {
        width: primary_monitor_size.width,
        height: primary_monitor_size.height,
    })))?;

    desktop_window.set_min_size(Some(tauri::Size::Physical(tauri::PhysicalSize {
        width: primary_monitor_size.width,
        height: primary_monitor_size.height,
    })))?;

    // for (index, monitor) in monitors.iter().enumerate() {
    //     let monitor_size = monitor.size();
    //     let monitor_position = monitor.position();

    //     let desktop_window = WebviewWindowBuilder::new(
    //         app,
    //         &format!("desktop_{}", index),
    //         WebviewUrl::App(format!("index.html#/desktop?monitor={}", index).into()),
    //     )
    //     .title(&format!("Vasak Desktop {}", index))
    //     .decorations(false)
    //     .position(monitor_position.x as f64, monitor_position.y as f64)
    //     .inner_size(monitor_size.width as f64, monitor_size.height as f64)
    //     .max_inner_size(monitor_size.width as f64, monitor_size.height as f64)
    //     .min_inner_size(monitor_size.width as f64, monitor_size.height as f64)
    //     .skip_taskbar(true)
    //     .build()?;

    //     set_window_properties(&desktop_window);
    // }

    Ok(())
}

fn set_window_properties(window: &tauri::WebviewWindow) {
    let gtk_window = window.gtk_window().expect("Failed to get GTK window");

    gtk_window.set_type_hint(gdk::WindowTypeHint::Desktop);
    gtk_window.set_skip_taskbar_hint(true);
}
