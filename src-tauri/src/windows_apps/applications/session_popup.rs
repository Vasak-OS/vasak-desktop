use gtk::prelude::*;
use tauri::{AppHandle, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::app_url::get_app_url;
use crate::gtk_utils;
use crate::monitor_manager::get_primary_monitor;

fn set_window_properties(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let gtk_window = window.gtk_window()?;

    unsafe {
        gtk_utils::invoke_on_main(move || {
            gtk_window.set_resizable(false);
            gtk_window.set_type_hint(gtk::gdk::WindowTypeHint::Dialog);
            gtk_window.set_urgency_hint(true);
            gtk_window.set_skip_taskbar_hint(true);
            gtk_window.set_skip_pager_hint(true);
            gtk_window.set_decorated(false);
            gtk_window.set_keep_above(true);
            gtk_window.stick();
        });
    }

    Ok(())
}

pub async fn create_session_popup_window(
    app: AppHandle,
    action: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;
    let monitor_size = primary_monitor.size();
    let monitor_position = primary_monitor.position();

    let window = WebviewWindowBuilder::new(
        &app,
        "session_popup",
        WebviewUrl::App(format!("index.html#/apps/session-popup?action={}", action).into()),
    )
    .title("Vasak Session")
    .decorations(false)
    .transparent(true)
    .inner_size(400.0, 280.0)
    .visible(true)
    .skip_taskbar(true)
    .always_on_top(true)
    .build()?;

    let win_for_blur = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Focused(false)) {
            let _ = win_for_blur.close();
        }
    });

    let complete_url = format!(
        "{}/index.html#/apps/session-popup?action={}",
        get_app_url(),
        action
    );
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = window.navigate(url);

    let center_x = monitor_position.x + (monitor_size.width as i32 / 2) - (400 / 2);
    let center_y = monitor_position.y + (monitor_size.height as i32 / 2) - (280 / 2);

    window
        .set_position(tauri::PhysicalPosition {
            x: center_x,
            y: center_y,
        })
        .map_err(|e| e.to_string())?;

    window.set_focus()?;

    let _ = set_window_properties(&window);

    Ok(())
}
