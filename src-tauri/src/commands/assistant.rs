//! El ciclo de vida de la superficie del asistente: abrirla, cerrarla y la
//! puerta de apagado. Espejo de `commands/control_center.rs`, con una
//! diferencia: antes de abrir se comprueba que el asistente esté encendido
//! (`assistant::is_available`), porque con el servicio apagado la superficie no
//! debe abrirse ni por el botón del panel ni por el atajo de Wayfire
//! (`OpenAssistant`).

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::surface_lifecycle::{
    cancel_terminate, clear_terminated, reload_surface, toggle_action, was_terminated, ToggleAction,
};
use crate::logger::{log_error, log_info, log_warning};
use crate::windows_apps::assistant::ASSISTANT_LABEL;
use crate::windows_apps::shell_layer::{
    hide_layer_window, layer_window_exists, layer_window_visible, show_layer_window,
};

/// Abre o cierra la superficie. Corre en el hilo principal de GTK, que es donde
/// vive el registro thread-local de superficies: leerlo desde otro hilo lo ve
/// vacío y el toggle no haría nada.
fn toggle_on_main(app: &AppHandle) {
    // Construida una vez al iniciar. Si eso falló, no existe: no hay nada que
    // mostrar.
    if !layer_window_exists(ASSISTANT_LABEL) {
        log_warning("[assistant] la superficie no existe; no se pudo crear al iniciar");
        return;
    }

    let visible = layer_window_visible(ASSISTANT_LABEL).unwrap_or(false);

    match toggle_action(visible, was_terminated(ASSISTANT_LABEL)) {
        ToggleAction::Hide => {
            log_info("[assistant] ocultando");
            hide_layer_window(ASSISTANT_LABEL);
        }
        ToggleAction::Show { reload } => {
            cancel_terminate(ASSISTANT_LABEL);
            // Si la muerte pendiente ya se llevó el proceso, se recarga para que
            // WebKit lo vuelva a levantar; la página se monta y vuelve a pedir
            // `History`.
            if reload {
                reload_surface(app, ASSISTANT_LABEL);
                clear_terminated(ASSISTANT_LABEL);
            }
            log_info("[assistant] mostrando");
            if let Some(webview) = app.get_webview_window(ASSISTANT_LABEL) {
                let _ = webview.emit("window-shown", ());
            }
            show_layer_window(ASSISTANT_LABEL);
        }
    }
}

/// La puerta de apagado más el toggle: comprueba disponibilidad (async, por
/// D-Bus) y, si el asistente está encendido, marshala el toggle al hilo
/// principal. La comparten el comando [`toggle_assistant`] y el método D-Bus
/// `OpenAssistant`.
pub(crate) async fn toggle_if_available(app: AppHandle) {
    if !crate::assistant::is_available(&app).await {
        log_info("[assistant] apagado: no se abre la superficie");
        return;
    }
    let for_main = app.clone();
    if let Err(error) = app.run_on_main_thread(move || toggle_on_main(&for_main)) {
        log_error(&format!(
            "[assistant] no se pudo alternar la superficie: {error}"
        ));
    }
}

/// Abre o cierra la superficie del asistente, si está encendido. Lo llama la
/// píldora del panel; con el asistente apagado no hace nada (y la píldora ni
/// siquiera se muestra, ver `assistant_available`).
#[tauri::command]
pub async fn toggle_assistant(app: AppHandle) {
    toggle_if_available(app).await;
}

/// Esconde la superficie sin preguntar si está abierta. La página se cierra con
/// esto —el botón, Escape—, no con el toggle: llamar al toggle sería la trampa
/// del centro de control, donde esconder dispara el cierre y el toggle la vuelve
/// a abrir. Esconder programa matar su WebKitWebProcess.
#[tauri::command]
pub fn hide_assistant() -> Result<(), ()> {
    hide_layer_window(ASSISTANT_LABEL);
    Ok(())
}
