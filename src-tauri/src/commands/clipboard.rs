use tauri::{AppHandle, Emitter, Manager};

use crate::logger::{log_info, log_warning};
use crate::windows_apps::clipboard::CLIPBOARD_LABEL;
use crate::windows_apps::shell_layer::{
    hide_layer_window, layer_window_exists, layer_window_visible, show_layer_window,
};

/// Abre o cierra la superposición del historial del portapapeles.
///
/// Igual que el centro de control: la superficie se construye una vez al iniciar,
/// sobre el hilo principal de GTK, y acá sólo se muestra o se esconde. La
/// visibilidad es de la superficie de capa, no de la ventana de Tauri (el webview
/// se reparentó dentro de ella).
#[tauri::command]
pub fn toggle_clipboard(app: AppHandle) -> Result<(), ()> {
    if !layer_window_exists(CLIPBOARD_LABEL) {
        log_warning("[clipboard] la superficie no existe; no se pudo crear al iniciar");
        return Ok(());
    }

    if layer_window_visible(CLIPBOARD_LABEL).unwrap_or(false) {
        log_info("[clipboard] ocultando");
        hide_layer_window(CLIPBOARD_LABEL);
        return Ok(());
    }

    log_info("[clipboard] mostrando");
    // La vista se refresca con este aviso en vez de rehacerse: la ventana se
    // esconde, nunca se destruye, así que Vue no se vuelve a montar.
    if let Some(webview) = app.get_webview_window(CLIPBOARD_LABEL) {
        let _ = webview.emit("window-shown", ());
    }
    show_layer_window(CLIPBOARD_LABEL);
    Ok(())
}

/// Esconde la superposición, sin preguntar si está abierta.
///
/// La página se cierra con esto y no con el conmutador: esconder saca el foco,
/// perder el foco cierra, y el conmutador la encontraría escondida y la abriría
/// de nuevo.
#[tauri::command]
pub fn hide_clipboard() -> Result<(), ()> {
    hide_layer_window(CLIPBOARD_LABEL);
    Ok(())
}
