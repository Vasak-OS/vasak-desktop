use gtk::prelude::*;
use tauri::{App, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::{monitor_manager::get_primary_monitor, windows_apps::menu};

pub fn create_menu(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(app.handle()).ok_or("No primary monitor found")?;
    let primary_monitor_size = primary_monitor.size();
    let primary_monitor_position = primary_monitor.position();

    let menu_window = app
        .get_webview_window("menu")
        .expect("menu window not found");

    menu_window.center()?;

    set_window_properties(&menu_window);

    let _ = menu_window.close();

    let _ = create_menu_window(app);
    Ok(())
}

pub fn create_menu_window(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let menu_window =
        WebviewWindowBuilder::from_config(app, app.config().app.windows.get(2).unwrap())?
            .title("Vasak Menu")
            .decorations(true)
            .transparent(false)
            .inner_size(900.0, 900.0)
            .visible(true)
            .skip_taskbar(false)
            .always_on_top(true)
            .build()?;

    menu_window.set_focus()?;
    menu_window.center()?;

    Ok(())
}

pub fn set_window_properties(window: &tauri::WebviewWindow) {
    let gtk_window = window.gtk_window().expect("Failed to get GTK window");

    gtk_window.set_type_hint(gdk::WindowTypeHint::Menu);
    gtk_window.set_skip_taskbar_hint(true);
}
