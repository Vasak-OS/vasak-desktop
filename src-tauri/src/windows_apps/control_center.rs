use gtk::prelude::*;
use tauri::{
    AppHandle, LogicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder,
};
use tokio::time::{sleep, Duration};


use crate::logger::log_info;
use crate::app_url::get_app_url;
use crate::gtk_utils;
use crate::monitor_manager::get_primary_monitor;
use crate::windows_apps::wayland_layer::{configure_wayland_layer, WaylandLayerMode};

fn set_window_properties(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let gtk_window = window.gtk_window()?;

    unsafe {
        gtk_utils::invoke_on_main(move || {
            gtk_window.set_resizable(false);
            gtk_window.set_type_hint(gtk::gdk::WindowTypeHint::Utility);
            gtk_window.set_urgency_hint(true);
            gtk_window.set_skip_taskbar_hint(true);
            gtk_window.set_skip_pager_hint(true);
            gtk_window.stick();
        });
    }

    Ok(())
}

pub async fn create_control_center_window(
    app: AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;

    // Tauri reports monitors in physical pixels, but Wayfire's output geometry
    // is logical. Feeding physical coordinates to configure_wayland_layer means
    // that on a scaled screen they fall outside every output, the lookup finds
    // no match and falls back to the first output — putting the control centre
    // on the wrong monitor. Work in logical pixels throughout.
    let scale = primary_monitor.scale_factor();
    let monitor_position = primary_monitor.position().to_logical::<i32>(scale);
    let monitor_size = primary_monitor.size().to_logical::<u32>(scale);

    let app_height = monitor_size.height.saturating_sub(60) as f64;
    let panel_width = 350;
    let right_x = monitor_position.x + monitor_size.width as i32 - panel_width - 10;
    let bottom_y = monitor_position.y + monitor_size.height as i32 - app_height as i32 - 10;

    log_info(&format!(
        "[control_center] primary monitor pos=({}, {}) size={}x{}, target=({}, {}) size={}x{}",
        monitor_position.x,
        monitor_position.y,
        monitor_size.width,
        monitor_size.height,
        right_x,
        bottom_y,
        panel_width,
        app_height
    ));

    let control_center_window = WebviewWindowBuilder::new(
        &app,
        "control_center",
        WebviewUrl::App("index.html#/control_center".into()),
    )
        .title("Vasak Control Center")
        .decorations(false)
        .transparent(true)
        .inner_size(panel_width as f64, app_height)
        .max_inner_size(panel_width as f64, app_height)
        .min_inner_size(panel_width as f64, app_height)
        .visible(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .build()?;

    let complete_url = format!("{}/index.html#/control_center", get_app_url());
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = control_center_window.navigate(url);

    control_center_window.set_position(Position::Logical(LogicalPosition {
        x: right_x as f64,
        y: bottom_y as f64,
    }))?;

    let _ = control_center_window.show();
    control_center_window.set_focus()?;

    configure_wayland_layer(
        "Vasak Control Center".to_string(),
        WaylandLayerMode::Panel,
        right_x,
        bottom_y,
        panel_width as u32,
        app_height as u32,
    );

    tauri::async_runtime::spawn(async move {
        sleep(Duration::from_millis(200)).await;
        configure_wayland_layer(
            "Vasak Control Center".to_string(),
            WaylandLayerMode::Panel,
            right_x,
            bottom_y,
            panel_width as u32,
            app_height as u32,
        );
    });

    let _ = set_window_properties(&control_center_window);

    Ok(())
}
