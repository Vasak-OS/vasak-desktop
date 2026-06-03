use gtk::prelude::*;
use tauri::{
    AppHandle, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tokio::time::{sleep, Duration};

use crate::logger::{log_info, log_warning};
use crate::app_url::get_app_url;
use crate::monitor_manager::get_primary_monitor;
use crate::windows_apps::wayland_layer::{configure_wayland_layer, WaylandLayerMode};

fn set_window_properties(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let gtk_window = window.gtk_window()?;

    gtk_window.set_resizable(false);
    gtk_window.set_type_hint(gtk::gdk::WindowTypeHint::Utility);
    gtk_window.set_urgency_hint(true);
    gtk_window.set_skip_taskbar_hint(true);
    gtk_window.set_skip_pager_hint(true);
    gtk_window.stick();

    Ok(())
}

pub async fn create_control_center_window(
    app: AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;
    let primary_monitor_size = primary_monitor.size();
    let app_height = primary_monitor_size.height.saturating_sub(60) as f64;
    let panel_width = 350;
    let monitor_position = primary_monitor.position();
    let monitor_size = primary_monitor_size;
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

    let control_center_window_for_blur = control_center_window.clone();
    control_center_window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Focused(false)) {
            let _ = control_center_window_for_blur.close();
        }
    });

    let complete_url = format!("{}/index.html#/control_center", get_app_url());
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = control_center_window.navigate(url);

    log_warning("[control_center] applying initial Tauri position before Wayfire geometry");
    control_center_window.set_position(Position::Physical(PhysicalPosition {
        x: right_x,
        y: bottom_y,
    }))?;

    log_info("[control_center] showing control center window");
    let _ = control_center_window.show();
    control_center_window.set_focus()?;

    log_info("[control_center] requesting Wayfire geometry apply (first pass)");
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
        log_info("[control_center] requesting Wayfire geometry apply (second pass)");
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
