use gtk::prelude::*;
use tauri::{AppHandle, Manager, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use std::sync::Arc;

use crate::app_url::get_app_url;
use crate::monitor_manager::get_primary_monitor;

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
    let app_height = (primary_monitor_size.height - 100).into();

    let control_center_window = Arc::new(
        WebviewWindowBuilder::new(
            &app,
            "control_center",
            WebviewUrl::App("index.html#/control_center".into()),
        )
        .title("Vasak Control Center")
        .decorations(false)
        .transparent(true)
        .inner_size(350.0, app_height)
        .max_inner_size(350.0, app_height)
        .min_inner_size(350.0, app_height)
        .visible(true)
        .skip_taskbar(true)
        .always_on_top(true)
        .build()?,
    );

    let complete_url = format!("{}/index.html#/control_center", get_app_url());
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = control_center_window.navigate(url);

    let monitor_size = primary_monitor.size();
    let monitor_position = primary_monitor.position();

    let right_x = monitor_position.x + monitor_size.width as i32 - 350 - 4; // 350 es el ancho de la ventana, 4 es el margen
    let center_y = monitor_position.y + 40 + (monitor_size.height as i32 / 2) - (app_height as i32 / 2);

    control_center_window.set_position(Position::Physical(PhysicalPosition {
        x: right_x,
        y: center_y,
    }))?;

    control_center_window.set_focus()?;

    set_window_properties(&control_center_window)?;

    let control_center_window_clone = Arc::clone(&control_center_window);

    control_center_window.on_window_event(move |event| match event {
        WindowEvent::Focused(is_focused) => {
            if !is_focused {
                let _ = control_center_window_clone.close();
            }
        }
        WindowEvent::CloseRequested { .. } => {
            let _ = control_center_window_clone.close();
        }
        _ => {}
    });

    Ok(())
}
