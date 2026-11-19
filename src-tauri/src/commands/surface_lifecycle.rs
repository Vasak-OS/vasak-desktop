//! Matar el `WebKitWebProcess` de una superficie de capa después de un rato
//! escondida, para liberar su RAM, y reponerlo recargando cuando vuelve a
//! mostrarse.
//!
//! El centro de control y el portapapeles se construyen una vez al iniciar y
//! dejaban su proceso de WebKit vivo toda la sesión —cada uno ~240 MB— aunque
//! casi nunca estén a la vista (vasak-desktop#307). La ventana y el widget siguen
//! vivos y escondidos; lo único que se libera es el proceso, que WebKit vuelve a
//! levantar al recargar la página al mostrarla.
//!
//! No se destruye la superficie: destruir un webview re-parentado deja colgado su
//! `WebKitWebProcess` bajo wry (vasak-desktop#264), así que destruir para liberar
//! memoria la fugaría en vez de bajarla. `terminate_web_process()` la baja de
//! verdad y no depende de wry.
//!
//! La gracia **no** es cero a propósito: matar el proceso dentro del propio `hide`
//! o del IPC que lo escondió recarga de más en reaperturas rápidas. Unos segundos
//! después, ya fuera de esa pila, se mata; si se reabre antes, no se toca y la
//! página se conserva.
//!
//! Vive en `commands/` —excluido de cobertura, como `webkit.rs`— porque el
//! cableado de glib/GTK no se puede montar sin un bucle principal real. La única
//! decisión pura, [`toggle_action`], se prueba igual acá abajo.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use tauri::{AppHandle, Manager};

/// Cuánto espera una superficie escondida antes de matar su proceso.
///
/// Corto: libera la RAM a los pocos segundos de dejar de usarse, sin matar en el
/// mismo tick (ver la nota de reentrancia arriba).
pub(crate) const GRACE: Duration = Duration::from_secs(3);

thread_local! {
    /// Muertes pendientes por etiqueta. `glib::SourceId` no es `Send`, así que el
    /// registro es del hilo principal, el único que lo toca.
    static PENDING: RefCell<HashMap<String, glib::SourceId>> = RefCell::new(HashMap::new());

    /// Superficies cuyo proceso ya se mató y que, por eso, hay que recargar antes
    /// de mostrarlas. Mismo hilo que `PENDING`.
    static TERMINATED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Lo que corresponde hacer al conmutar una superficie, según esté a la vista y
/// según su proceso siga vivo.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ToggleAction {
    /// Esconderla y programar la muerte de su proceso.
    Hide,
    /// Mostrarla; `reload` es `true` si su proceso se mató y hay que recargar la
    /// página para que WebKit lo vuelva a levantar.
    Show { reload: bool },
}

/// Decide qué hacer al conmutar, a partir de si está visible y si su proceso se
/// mató.
///
/// Pura y por eso testeable: el resto del módulo es glue de GTK que no lo es.
pub(crate) fn toggle_action(visible: bool, terminated: bool) -> ToggleAction {
    if visible {
        ToggleAction::Hide
    } else {
        ToggleAction::Show { reload: terminated }
    }
}

/// Si el proceso de `label` se mató mientras estaba escondida. Lo lee el comando,
/// que corre en el hilo principal, para decidir si recargar al mostrar.
pub(crate) fn was_terminated(label: &str) -> bool {
    TERMINATED
        .try_with(|t| t.borrow().contains(label))
        .unwrap_or(false)
}

/// Mata el `WebKitWebProcess` de la superficie `label` sin tocar su ventana ni su
/// widget, que siguen vivos y escondidos. WebKit lo respawnea al recargar con
/// [`reload_surface`]. Llega al webview con `with_webview`, que devuelve el
/// `WebKitWebView` que Tauri guarda aunque se haya re-parentado en la ventana de
/// capa; por eso no hace falta bajar por el árbol de GTK ni tocar el registro de
/// capas. Sólo hilo principal de GTK.
fn terminate(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.with_webview(|platform| {
            use webkit2gtk::WebViewExt;
            platform.inner().terminate_web_process();
        });
    }
}

/// Recarga la superficie `label`, con lo que WebKit respawnea el proceso que
/// [`terminate`] mató. La página se monta de nuevo y vuelve a pedir su estado.
/// Sólo hilo principal.
pub(crate) fn reload_surface(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.with_webview(|platform| {
            use webkit2gtk::WebViewExt;
            platform.inner().reload();
        });
    }
}

/// ¿La superficie `label` está a la vista? La consulta la página al montar para
/// saber si ponerse en estado «mostrada»: tras matar su proceso y recargarla, el
/// `window-shown` que se emite al mostrarla corre contra el remonte de Vue y se
/// pierde, así que la página recargada lo pregunta en vez de esperarlo. Corre en
/// el hilo principal de GTK —como los `toggle_*`, que leen el registro igual—,
/// donde vive el registro de capas.
#[tauri::command]
pub fn surface_is_shown(label: String) -> bool {
    crate::windows_apps::shell_layer::layer_window_visible(&label).unwrap_or(false)
}

/// Programa matar el proceso de `label` tras [`GRACE`] escondida. Reemplaza
/// cualquier muerte ya pendiente para esa superficie. Seguro desde cualquier
/// hilo: marshala al hilo principal de GTK.
pub(crate) fn arm_terminate_after_hide(app: &AppHandle, label: &str) {
    let app = app.clone();
    let label = label.to_string();
    unsafe {
        crate::gtk_utils::invoke_on_main(move || {
            cancel_pending(&label);
            let for_timer = label.clone();
            let for_map = label.clone();
            let source = glib::timeout_add_local_once(GRACE, move || {
                terminate(&app, &for_timer);
                TERMINATED.with(|t| {
                    t.borrow_mut().insert(for_timer.clone());
                });
                PENDING.with(|p| {
                    p.borrow_mut().remove(&for_timer);
                });
            });
            PENDING.with(|p| {
                p.borrow_mut().insert(for_map, source);
            });
        });
    }
}

/// Cancela una muerte pendiente. Se llama al mostrar la superficie otra vez: si
/// la muerte todavía no corrió, el proceso sigue vivo y la página se conserva.
/// Seguro desde cualquier hilo.
pub(crate) fn cancel_terminate(label: &str) {
    let label = label.to_string();
    unsafe {
        crate::gtk_utils::invoke_on_main(move || cancel_pending(&label));
    }
}

/// Marca que `label` ya se recargó y su proceso volvió. Sólo hilo principal.
pub(crate) fn clear_terminated(label: &str) {
    TERMINATED.with(|t| {
        t.borrow_mut().remove(label);
    });
}

/// Saca la entrada del registro y da de baja su temporizador. Sólo hilo principal.
fn cancel_pending(label: &str) {
    PENDING.with(|p| {
        if let Some(source) = p.borrow_mut().remove(label) {
            source.remove();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_se_esconde() {
        assert_eq!(toggle_action(true, false), ToggleAction::Hide);
        // Da igual si su proceso se mató: si está a la vista, está corriendo.
        assert_eq!(toggle_action(true, true), ToggleAction::Hide);
    }

    #[test]
    fn escondida_con_proceso_vivo_solo_se_muestra() {
        assert_eq!(
            toggle_action(false, false),
            ToggleAction::Show { reload: false }
        );
    }

    #[test]
    fn escondida_con_proceso_muerto_se_recarga_antes_de_mostrar() {
        // Es el caso que trae el fix: la muerte pendiente ya corrió, así que
        // mostrarla sin recargar dejaría el webview sin proceso.
        assert_eq!(
            toggle_action(false, true),
            ToggleAction::Show { reload: true }
        );
    }
}
