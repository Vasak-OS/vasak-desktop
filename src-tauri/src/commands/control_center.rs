use tauri::{AppHandle, Emitter, Manager};

use crate::commands::surface_lifecycle::{
    cancel_terminate, clear_terminated, reload_surface, toggle_action, was_terminated, ToggleAction,
};
use crate::logger::log_info;
use crate::windows_apps::control_center::CONTROL_CENTER_LABEL;
use crate::windows_apps::shell_layer::{
    hide_layer_window, layer_window_visible, show_layer_window,
};

#[tauri::command]
pub fn toggle_control_center(app: AppHandle) -> Result<(), ()> {
    // Visibility is a property of the layer surface, not of Tauri's window: the
    // webview was reparented into it, and the toplevel Tauri built is hidden
    // for good. Asking the wrong one is how the toggle got out of step with
    // what was on screen.
    let visible = layer_window_visible(CONTROL_CENTER_LABEL).unwrap_or(false);

    match toggle_action(visible, was_terminated(CONTROL_CENTER_LABEL)) {
        ToggleAction::Hide => {
            log_info("[control_center] ocultando");
            // Esconder dispara `on_hide`, que programa matar su WebKitWebProcess
            // a los pocos segundos (vasak-desktop#307).
            hide_layer_window(CONTROL_CENTER_LABEL);
            crate::night_light::unwatch(&app);
        }
        ToggleAction::Show { reload } => {
            cancel_terminate(CONTROL_CENTER_LABEL);
            // Si la muerte pendiente ya se llevó el proceso, se recarga para que
            // WebKit lo vuelva a levantar; la página se monta de nuevo y vuelve a
            // pedir su estado.
            if reload {
                reload_surface(&app, CONTROL_CENTER_LABEL);
                clear_terminated(CONTROL_CENTER_LABEL);
            }
            log_info("[control_center] mostrando");
            if let Some(webview) = app.get_webview_window(CONTROL_CENTER_LABEL) {
                let _ = webview.emit("window-shown", ());
            }
            show_layer_window(CONTROL_CENTER_LABEL);
            // La luz nocturna se sigue sólo con el centro a la vista: las señales
            // de systemd no le sirven a nadie con el centro cerrado.
            crate::night_light::watch(&app);
        }
    }

    Ok(())
}

/// Hides the centre, without asking whether it is open.
///
/// The page closes itself with this rather than with the toggle. Calling the
/// toggle is the trap the menu fell into: hiding drops focus, focus loss closes
/// it, and the toggle then finds it hidden and opens it again.
#[tauri::command]
pub fn hide_control_center(app: AppHandle) -> Result<(), ()> {
    // Esconder dispara `on_hide`, que programa matar el WebKitWebProcess.
    hide_layer_window(CONTROL_CENTER_LABEL);
    crate::night_light::unwatch(&app);
    Ok(())
}
