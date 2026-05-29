use crate::app_url::get_app_url;
use gtk::prelude::*;
use gtk_layer_shell::{Edge, Layer, LayerShell};
use tauri::{
    async_runtime::spawn, App, Monitor, Url, WebviewUrl, WebviewWindowBuilder,
};

use crate::monitor_manager::{get_monitors, get_primary_monitor};

pub fn create_desktops(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let monitors = get_monitors(app.handle()).ok_or("No monitors found")?;
    let primary_monitor = get_primary_monitor(app.handle()).ok_or("No primary monitor found")?;

    let primary_monitor_size = primary_monitor.size();
    let primary_monitor_position = primary_monitor.position();

    let primary_desktop_window = WebviewWindowBuilder::new(
        app,
        "desktop",
        WebviewUrl::App("index.html#/desktop".into()),
    )
    .title("Vasak Desktop")
    .decorations(false)
    .transparent(true)
    .inner_size(primary_monitor_size.width as f64, primary_monitor_size.height as f64)
    .max_inner_size(primary_monitor_size.width as f64, primary_monitor_size.height as f64)
    .min_inner_size(primary_monitor_size.width as f64, primary_monitor_size.height as f64)
    .position(primary_monitor_position.x as f64, primary_monitor_position.y as f64)
    .visible(false)
    .skip_taskbar(true)
    .always_on_bottom(true)
    .build()?;

    set_window_properties(
        &primary_desktop_window,
        "Vasak Desktop".to_string(),
        primary_monitor_position.x,
        primary_monitor_position.y,
        primary_monitor_size.width,
        primary_monitor_size.height,
    );

    for (index, monitor) in monitors.iter().enumerate() {
        if monitor.position() == primary_monitor_position {
            continue; // Skip the primary monitor
        }

        let app_handle = app.handle().clone();
        let monitor_clone = monitor.clone();

        spawn(async move {
            let _ = open_other_desktop(app_handle, index, monitor_clone).await;
        });
    }

    Ok(())
}

async fn open_other_desktop(app_handle: tauri::AppHandle, index: usize, monitor: Monitor) {
    let label = format!("desktop_{}", index);

    let monitor_size = monitor.size();
    let monitor_position = monitor.position();

    let other_desktop_window = WebviewWindowBuilder::new(
        &app_handle,
        &label,
        WebviewUrl::App(format!("index.html#/desktop?monitor={}", label).into()),
    )
    .title(format!("Vasak Desktop {}", index))
    .decorations(false)
    .transparent(false)
    .inner_size(monitor_size.width as f64, monitor_size.height as f64)
    .max_inner_size(monitor_size.width as f64, monitor_size.height as f64)
    .min_inner_size(monitor_size.width as f64, monitor_size.height as f64)
    .position(monitor_position.x as f64, monitor_position.y as f64)
    .visible(false)
    .skip_taskbar(true)
    .always_on_bottom(true)
    .build();

    if let Ok(other_desktop_window) = other_desktop_window {
 
    let complete_url = format!("{}index.html#/desktop?monitor={}", get_app_url(), label);
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = other_desktop_window.navigate(url);

    set_window_properties(
        &other_desktop_window,
        format!("Vasak Desktop {}", index),
        monitor_position.x,
        monitor_position.y,
        monitor_size.width,
        monitor_size.height,
    );
    } else {
        eprintln!("Failed to create desktop window for monitor {}", index);
    }
}

fn set_window_properties(
    window: &tauri::WebviewWindow,
    _title: String,
    _x: i32,
    _y: i32,
    _width: u32,
    _height: u32,
) {
    let gtk_window = window.gtk_window().expect("Failed to get GTK window");

    gtk_window.set_type_hint(gdk::WindowTypeHint::Desktop);
    gtk_window.set_accept_focus(false);

    if gtk_layer_shell::is_supported() {
        gtk_window.init_layer_shell();
        gtk_window.set_namespace("vasak-desktop");
        gtk_window.set_layer(Layer::Background);
        gtk_window.set_anchor(Edge::Top, true);
        gtk_window.set_anchor(Edge::Left, true);
        gtk_window.set_anchor(Edge::Right, true);
        gtk_window.set_anchor(Edge::Bottom, true);
        gtk_window.set_keyboard_interactivity(false);
        gtk_window.set_exclusive_zone(0);
    }

    #[cfg(feature = "x11")]
    gtk_window.set_skip_taskbar_hint(true);

    let _ = window.show();
    gtk_window.show_all();
    gtk_window.present();
}
