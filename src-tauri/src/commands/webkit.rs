//! El re-parenteo del webview a la ventana de capa y el afinado de WebKit.
//!
//! Aparte del resto a propósito: es la capa que habla con GTK y con el
//! `WebKitWebView`, y no se puede ejercitar sin un display ni un WebKit reales.
//! Por eso vive en `commands/`, que el análisis de cobertura del taller excluye
//! —igual que los `.vue`—: medir cobertura de lo que ninguna prueba puede montar
//! sólo bloquea sin informar. Lo testeable (el cálculo del lugar de los applets,
//! el tamaño del panel) se queda en sus módulos.

use gtk::prelude::*;

/// Afina el WebKit de una superficie residente para bajar su consumo.
///
/// Cada superficie del escritorio deja un `WebKitWebProcess` vivo mientras la
/// sesión dura —se escabullen y se reusan, no se recrean— y hasta ahora todas
/// corrían con los valores por omisión de WebKitGTK (vasak-desktop#265). Esto
/// baja el piso de cada una con las palancas **seguras** para una superficie que
/// anima: sin las extras de desarrollo, sin los gestos de navegación
/// atrás/adelante (no navega a ningún lado) y con el caché de un visor de
/// documento, no el de un navegador.
///
/// **No** se toca la aceleración por hardware ni WebGL: los applets, el menú y el
/// centro de control animan su entrada y su salida, y apagar la GPU ahí haría
/// tironear eso. Ésas son las palancas que el perfil de vapp#43 reserva para las
/// superficies estáticas.
fn tune_webkit(webview: &webkit2gtk::WebView) {
    use webkit2gtk::{CacheModel, SettingsExt, WebContextExt, WebViewExt};

    if let Some(settings) = WebViewExt::settings(webview) {
        settings.set_enable_developer_extras(false);
        settings.set_enable_back_forward_navigation_gestures(false);
    }
    if let Some(context) = WebViewExt::web_context(webview) {
        // No navega páginas: con el caché mínimo alcanza.
        context.set_cache_model(CacheModel::DocumentViewer);
    }
}

/// Moves the WebKit widget out of Tauri's toplevel and into the layer window.
///
/// De paso lo afina: es el único camino por el que pasan todas las superficies de
/// capa (panel, escritorio, centro de control, selector de fondo y los applets),
/// así que afinar acá las cubre a todas en un solo lugar.
pub(crate) fn reparent_webview(
    gtk_window: &gtk::ApplicationWindow,
    layer_win: &gtk::Window,
) -> Result<(), Box<dyn std::error::Error>> {
    let child = gtk_window
        .child()
        .ok_or("Tauri window has no child widget")?;

    let container = child
        .dynamic_cast_ref::<gtk::Container>()
        .ok_or_else(|| format!("Tauri child {} is not a container", child.type_().name()))?;

    let widget = container
        .children()
        .first()
        .cloned()
        .ok_or("Tauri container holds no webview")?;

    if let Ok(webview) = widget.clone().dynamic_cast::<webkit2gtk::WebView>() {
        tune_webkit(&webview);
    }

    container.remove(&widget);
    layer_win.add(&widget);

    Ok(())
}

/// Afina el WebKit de una ventana de Tauri común —no re-parentada—: el OSD, el
/// menú del teléfono y el diálogo de sesión.
///
/// Llega al `WebKitWebView` bajando por el árbol de GTK, así que el trabajo va en
/// el hilo principal; obtener el `gtk::Window` desde el comando sí es seguro.
pub(crate) fn tune_window(window: &tauri::WebviewWindow) {
    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };
    unsafe {
        crate::gtk_utils::invoke_on_main(move || {
            if let Some(child) = gtk_window.child() {
                if let Some(container) = child.dynamic_cast_ref::<gtk::Container>() {
                    if let Some(widget) = container.children().into_iter().next() {
                        if let Ok(webview) = widget.dynamic_cast::<webkit2gtk::WebView>() {
                            tune_webkit(&webview);
                        }
                    }
                }
            }
        });
    }
}
