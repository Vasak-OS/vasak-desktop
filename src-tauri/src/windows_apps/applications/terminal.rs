use gtk::prelude::*;
use std::sync::Arc;
use tauri::{AppHandle, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder};

use crate::{app_url::get_app_url, monitor_manager::get_primary_monitor};

pub fn create_app_terminal_window(
    app: AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;

    let app_terminal_window = Arc::new(
        WebviewWindowBuilder::new(
            &app,
            "app_terminal",
            WebviewUrl::App("index.html#/apps/terminal".into()),
        )
        .title("Vasak Terminal")
        .decorations(false)
        .transparent(true)
        .inner_size(700.0, 620.0)
        .visible(true)
        .build()?,
    );

    let complete_url = format!("{}/index.html#/apps/terminal", get_app_url());
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = app_terminal_window.navigate(url);

    let monitor_size = primary_monitor.size();
    let monitor_position = primary_monitor.position();

    let center_x = monitor_position.x + (monitor_size.width as i32 / 2) - (700 / 2);
    let center_y = monitor_position.y + (monitor_size.height as i32 / 2) - (620 / 2);

    app_terminal_window.set_position(Position::Physical(PhysicalPosition {
        x: center_x,
        y: center_y,
    }))?;

    app_terminal_window.set_focus()?;

    Ok(())
}
