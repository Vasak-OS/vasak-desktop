use gtk::prelude::*;
use tauri::{App, AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::app_url::get_app_url;

pub fn create_menu(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let menu_window = app
        .get_webview_window("menu")
        .expect("menu window not found");

    let _ = menu_window.close();

    Ok(())
}

pub async fn create_menu_window(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let menu_window =
        WebviewWindowBuilder::new(&app, "menu", WebviewUrl::App("index.html#/menu".into()))
            .title("Vasak Menu")
            .decorations(true)
            .transparent(false)
            .inner_size(900.0, 620.0)
            .max_inner_size(900.0, 620.0)
            .min_inner_size(900.0, 620.0)
            .visible(true)
            .skip_taskbar(false)
            .always_on_top(true)
            .build()?;

    menu_window.on_window_event(move |event| match event {
        WindowEvent::Focused(is_focused) => {
            if !is_focused {
                std::process::exit(0);
            }
        }
        WindowEvent::CloseRequested { .. } => {
            std::process::exit(0);
        }
        _ => {}
    });

    let complete_url = format!("{}{}", get_app_url(), "index.html#/menu");
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = menu_window.navigate(url);

    menu_window.center()?;
    menu_window.set_focus()?;

    Ok(())
}

pub fn set_window_properties(window: &tauri::WebviewWindow) {
    let gtk_window = window.gtk_window().expect("Failed to get GTK window");

    gtk_window.set_type_hint(gdk::WindowTypeHint::Menu);
    gtk_window.set_skip_taskbar_hint(true);
}
