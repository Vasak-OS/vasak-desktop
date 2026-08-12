use tauri::{AppHandle, Emitter, Manager};

use crate::logger::{log_info, log_warning};
use crate::windows_apps::control_center::CONTROL_CENTER_LABEL;
use crate::windows_apps::shell_layer::{
    hide_layer_window, layer_window_exists, layer_window_visible, show_layer_window,
};

#[tauri::command]
pub fn toggle_control_center(app: AppHandle) -> Result<(), ()> {
    // Built once at startup on the GTK main thread. Building it here instead
    // would mean touching GTK from whatever thread the command landed on, which
    // is not thread-safe and killed the process.
    if !layer_window_exists(CONTROL_CENTER_LABEL) {
        log_warning("[control_center] la superficie no existe; no se pudo crear al iniciar");
        return Ok(());
    }

    // Visibility is a property of the layer surface, not of Tauri's window: the
    // webview was reparented into it, and the toplevel Tauri built is hidden
    // for good. Asking the wrong one is how the toggle got out of step with
    // what was on screen.
    if layer_window_visible(CONTROL_CENTER_LABEL).unwrap_or(false) {
        log_info("[control_center] ocultando");
        hide_layer_window(CONTROL_CENTER_LABEL);
    } else {
        log_info("[control_center] mostrando");
        // The view refreshes on this rather than on being rebuilt: the window is
        // hidden, never destroyed, so the page is not reloaded and Vue does not
        // re-run.
        if let Some(webview) = app.get_webview_window(CONTROL_CENTER_LABEL) {
            let _ = webview.emit("window-shown", ());
        }
        show_layer_window(CONTROL_CENTER_LABEL);
    }

    Ok(())
}
