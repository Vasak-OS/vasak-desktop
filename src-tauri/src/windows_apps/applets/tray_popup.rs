use gtk::prelude::*;
use tauri::{AppHandle, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder};

use crate::{app_url::get_app_url, monitor_manager::get_primary_monitor};

pub async fn create_systray_popup_window(
    app: AppHandle,
    width: f64,
    height: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;

    let window = WebviewWindowBuilder::new(
        &app,
        "systray_popup",
        WebviewUrl::App("index.html#/applets/tray-popup".into()),
    )
    .title("")
    .decorations(false)
    .transparent(true)
    .inner_size(width, height)
    .max_inner_size(width, height)
    .min_inner_size(width, height)
    .visible(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .build()?;

    let complete_url = format!("{}/index.html#/applets/tray-popup", get_app_url());
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = window.navigate(url);

    let monitor_size = primary_monitor.size();
    let monitor_position = primary_monitor.position();

    let center_x = monitor_position.x + (monitor_size.width as i32 / 2) - (width as i32 / 2);
    let center_y = monitor_position.y + (monitor_size.height as i32 / 2) - (height as i32 / 2);

    window.set_position(Position::Physical(PhysicalPosition {
        x: center_x,
        y: center_y,
    }))?;

    set_window_properties(&window);

    let _ = window.show();
    window.set_focus()?;

    Ok(())
}

fn set_window_properties(window: &tauri::WebviewWindow) {
    let window = window.clone();
    glib::MainContext::default().invoke(move || {
        if let Ok(gtk_window) = window.gtk_window() {
            gtk_window.set_type_hint(gdk::WindowTypeHint::Utility);
            gtk_window.set_skip_taskbar_hint(true);
        }
    });
}
