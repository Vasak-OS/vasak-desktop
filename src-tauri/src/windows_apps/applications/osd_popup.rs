use gtk::prelude::*;
use tauri::{AppHandle, Url, WebviewUrl, WebviewWindowBuilder};
use url::form_urlencoded::Serializer;

use crate::app_url::get_app_url;
use crate::gtk_utils;
use crate::monitor_manager::get_primary_monitor;

fn set_window_properties(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let gtk_window = window.gtk_window()?;

    unsafe {
        gtk_utils::invoke_on_main(move || {
            gtk_window.set_resizable(false);
            gtk_window.set_decorated(false);
        });
    }

    Ok(())
}

pub async fn create_osd_window(
    app: &AppHandle,
    icon: &str,
    value: f64,
    maximum: f64,
    label: &str,
) -> Result<tauri::WebviewWindow, Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(app).ok_or("No primary monitor found")?;
    let monitor_size = primary_monitor.size();
    let monitor_position = primary_monitor.position();

    let params = Serializer::new(String::new())
        .append_pair("icon", icon)
        .append_pair("value", &value.to_string())
        .append_pair("maximum", &maximum.to_string())
        .append_pair("label", label)
        .finish();

    let window = WebviewWindowBuilder::new(
        app,
        "osd_popup",
        WebviewUrl::App(format!("index.html#/apps/osd-popup?{}", params).into()),
    )
    .title("")
    .decorations(false)
    .transparent(true)
    .inner_size(220.0, 120.0)
    .visible(false)
    .skip_taskbar(true)
    .build()?;

    let complete_url = format!("{}/index.html#/apps/osd-popup?{}", get_app_url(), params);
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = window.navigate(url);

    let center_x = monitor_position.x + (monitor_size.width as i32 / 2) - (320 / 2);
    let center_y = monitor_position.y + (monitor_size.height as i32 / 3) - (140 / 2);

    window.set_position(tauri::PhysicalPosition { x: center_x, y: center_y })?;

    let _ = set_window_properties(&window);

    Ok(window)
}
