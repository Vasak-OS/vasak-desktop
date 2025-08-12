use gtk::prelude::*;
use std::sync::Arc;
use tauri::{App, AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::{app_url::get_app_url};

pub fn create_menu(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let menu_window = app
        .get_webview_window("menu")
        .expect("menu window not found");

    let _ = menu_window.close();

    Ok(())
}

pub async fn create_menu_window(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let menu_window = Arc::new(
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
            .build()?,
    );

    let complete_url = format!("{}/index.html#/menu", get_app_url());
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = menu_window.navigate(url);

    menu_window.center()?;
    menu_window.set_focus()?;

    set_window_properties(&menu_window);

    let menu_window_clone = Arc::clone(&menu_window);
    menu_window.on_window_event(move |event| {
        match event {
            WindowEvent::Focused(is_focused) => {
                if !is_focused {
                    let _ = menu_window_clone.close();
                }
            }
            WindowEvent::CloseRequested { .. } => {
                let _ = menu_window_clone.close();
            }
            _ => {}
        }
    });

    Ok(())
}

fn set_window_properties(window: &tauri::WebviewWindow) {
    let gtk_window = window.gtk_window().expect("Failed to get GTK window");

    gtk_window.set_type_hint(gdk::WindowTypeHint::Menu);
    gtk_window.set_skip_taskbar_hint(true);
}
