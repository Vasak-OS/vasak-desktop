use gtk::prelude::*;
use std::sync::Arc;
use tauri::{AppHandle, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder};

use crate::app_url::get_app_url;
use crate::monitor_manager::get_primary_monitor;

fn set_window_properties(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let gtk_window = window.gtk_window()?;

    gtk_window.set_resizable(false);
    gtk_window.set_type_hint(gtk::gdk::WindowTypeHint::Dialog);
    gtk_window.set_urgency_hint(true);
    gtk_window.set_skip_taskbar_hint(true);
    gtk_window.set_skip_pager_hint(true);
    gtk_window.set_decorated(false);
    gtk_window.set_keep_above(true);
    gtk_window.stick();

    Ok(())
}

pub async fn create_search_window(
    app: AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;
    let primary_monitor_size = primary_monitor.size();

    let search_window = Arc::new(
        WebviewWindowBuilder::new(
            &app,
            "app_search",
            WebviewUrl::App("index.html#/apps/search".into()),
        )
        .title("Vasak Search")
        .decorations(false)
        .transparent(true)
        .inner_size(700.0, 600.0)
        .visible(true)
        .skip_taskbar(true)
        .always_on_top(true)
        .center()
        .build()?,
    );

    let complete_url = format!("{}/index.html#/apps/search", get_app_url());
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = search_window.navigate(url);

    search_window.set_focus()?;

    let _ = set_window_properties(&search_window);

    Ok(())
}
