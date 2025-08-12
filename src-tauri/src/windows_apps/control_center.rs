use gtk::prelude::*;
use tauri::Manager;

fn set_window_properties(window: &tauri::WebviewWindow) {
    let gtk_window = window.gtk_window()?;

    gtk_window.set_resizable(false);
    gtk_window.set_type_hint(gtk::gdk::WindowTypeHint::Utility);
    gtk_window.set_urgency_hint(true);
    gtk_window.set_skip_taskbar_hint(true);
    gtk_window.set_skip_pager_hint(true);
    gtk_window.stick();

    Ok(())
}