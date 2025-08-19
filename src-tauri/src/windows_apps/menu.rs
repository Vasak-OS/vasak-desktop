use gtk::prelude::*;
use std::sync::Arc;
use tauri::{
    AppHandle, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

use crate::{app_url::get_app_url, monitor_manager::get_primary_monitor};

pub async fn create_menu_window(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;

    let menu_window = Arc::new(
        WebviewWindowBuilder::new(&app, "menu", WebviewUrl::App("index.html#/menu".into()))
            .title("Vasak Menu")
            .decorations(false)
            .transparent(true)
            .inner_size(900.0, 620.0)
            .max_inner_size(900.0, 620.0)
            .min_inner_size(900.0, 620.0)
            .visible(true)
            .skip_taskbar(true)
            .always_on_top(true)
            .build()?,
    );

    let complete_url = format!("{}/index.html#/menu", get_app_url());
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = menu_window.navigate(url);

    let monitor_size = primary_monitor.size();
    let monitor_position = primary_monitor.position();

    let center_x = monitor_position.x + (monitor_size.width as i32 / 2) - (900 / 2);
    let center_y = monitor_position.y + (monitor_size.height as i32 / 2) - (620 / 2);

    menu_window.set_position(Position::Physical(PhysicalPosition {
        x: center_x,
        y: center_y,
    }))?;

    menu_window.set_focus()?;

    set_window_properties(&menu_window);

    let menu_window_clone = Arc::clone(&menu_window);

    menu_window.on_window_event(move |event| match event {
        WindowEvent::Focused(is_focused) => {
            if !is_focused {
                let _ = menu_window_clone.close();
            }
        }
        WindowEvent::CloseRequested { .. } => {
            let _ = menu_window_clone.close();
        }
        _ => {}
    });

    Ok(())
}

fn set_window_properties(window: &tauri::WebviewWindow) {
    let gtk_window = window.gtk_window().expect("Failed to get GTK window");

    gtk_window.set_type_hint(gdk::WindowTypeHint::Menu);
    gtk_window.set_skip_taskbar_hint(true);
}
