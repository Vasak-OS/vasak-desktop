use tauri::{AppHandle, Emitter, Manager};

use crate::commands::surface_lifecycle::{
    cancel_terminate, clear_terminated, reload_surface, toggle_action, was_terminated, ToggleAction,
};
use crate::logger::log_info;
use crate::windows_apps::clipboard::CLIPBOARD_LABEL;
use crate::windows_apps::shell_layer::{
    hide_layer_window, layer_window_visible, show_layer_window,
};

/// Abre o cierra la superposición del historial del portapapeles.
///
/// Igual que el centro de control: la superficie arranca construida y escondida,
/// y acá se muestra o se esconde sobre el hilo principal de GTK. Al esconderse se
/// programa matar su WebKitWebProcess para no dejarlo vivo toda la sesión; al
/// mostrarse se cancela, y se recarga si la muerte ya corrió.
#[tauri::command]
pub fn toggle_clipboard(app: AppHandle) -> Result<(), ()> {
    let visible = layer_window_visible(CLIPBOARD_LABEL).unwrap_or(false);

    match toggle_action(visible, was_terminated(CLIPBOARD_LABEL)) {
        ToggleAction::Hide => {
            log_info("[clipboard] ocultando");
            // Esconder dispara `on_hide`, que programa matar su WebKitWebProcess.
            hide_layer_window(CLIPBOARD_LABEL);
        }
        ToggleAction::Show { reload } => {
            cancel_terminate(CLIPBOARD_LABEL);
            if reload {
                reload_surface(&app, CLIPBOARD_LABEL);
                clear_terminated(CLIPBOARD_LABEL);
            }
            log_info("[clipboard] mostrando");
            if let Some(webview) = app.get_webview_window(CLIPBOARD_LABEL) {
                let _ = webview.emit("window-shown", ());
            }
            show_layer_window(CLIPBOARD_LABEL);
        }
    }
    Ok(())
}

/// Esconde la superposición, sin preguntar si está abierta.
///
/// La página se cierra con esto y no con el conmutador: esconder saca el foco,
/// perder el foco cierra, y el conmutador la encontraría escondida y la abriría
/// de nuevo.
#[tauri::command]
pub fn hide_clipboard(_app: AppHandle) -> Result<(), ()> {
    // Esconder dispara `on_hide`, que programa matar el WebKitWebProcess.
    hide_layer_window(CLIPBOARD_LABEL);
    Ok(())
}
