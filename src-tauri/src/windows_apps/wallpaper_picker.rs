//! El selector rápido de fondos: una superposición a pantalla completa sobre el
//! monitor principal (vasak-desktop#133).
//!
//! # Por qué superposición y no applet anclado
//!
//! El issue dejaba las dos. El punto de elegir el fondo desde el escritorio es
//! verlo detrás, y la referencia (§6 de `video-reference.md`) lo muestra así: una
//! fila de tarjetas a media altura **sin contenedor**. Un applet colgado del panel
//! lo metería en una caja chica y taparía justo lo que se quiere mirar. Así que
//! va como el centro de control —una superficie de capa propia, `Overlay`, sin
//! reservar espacio— pero anclada a los cuatro bordes.
//!
//! # Se construye al pedirla
//!
//! El centro de control se arma al arrancar porque se abre seguido; esto se abre
//! de vez en cuando, y un WebView más esperando escondido son decenas de megas.
//! Se arma la primera vez, **en el hilo principal de GTK** (`run_on_main_thread`
//! en quien llama: tocar GTK desde un hilo de Tokio tumbó el proceso una vez,
//! ver `lib.rs`), y desde ahí se esconde y se muestra.
//!
//! # Teclado
//!
//! `OnDemand` y `dismiss_on_unfocus`, como el centro de control: Escape y hacer
//! clic en otra ventana la cierran desde GTK, aunque la página no llegue a
//! enterarse.

use gtk_layer_shell::{KeyboardMode, Layer};
use tauri::{AppHandle, Emitter, Manager};

use crate::logger::log_info;
use crate::monitor_manager::{find_gdk_monitor, get_primary_monitor};
use crate::windows_apps::shell_layer::{spawn_layer_window, LayerSpec};

pub const WALLPAPER_PICKER_LABEL: &str = "wallpaper_picker";

/// El espacio de nombres de la capa: propio, para que una regla de Wayfire
/// (el desenfoque, una animación) pueda reconocer al selector.
pub const WALLPAPER_PICKER_NAMESPACE: &str = "vasak-wallpaper-picker";

/// Lo que se le pide a la superficie: los cuatro bordes, sin reservar nada.
pub fn picker_spec() -> LayerSpec {
    LayerSpec {
        namespace: WALLPAPER_PICKER_NAMESPACE,
        // Encima del panel: el velo cubre la pantalla entera.
        layer: Layer::Overlay,
        anchors: (true, true, true, true),
        // Las ventanas no se corren por algo que aparece y desaparece.
        exclusive_zone: Some(-1),
        keyboard: KeyboardMode::OnDemand,
        dismiss_on_unfocus: true,
        ..Default::default()
    }
}

/// Arma la superficie, visible. Sólo desde el hilo principal de GTK.
pub fn create_wallpaper_picker_window(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let primary = get_primary_monitor(app).ok_or("No primary monitor found")?;
    let gdk_monitor =
        find_gdk_monitor(&primary).ok_or("No GDK monitor matching the primary monitor")?;

    let scale = primary.scale_factor();
    let size = (
        primary.size().width as f64 / scale,
        primary.size().height as f64 / scale,
    );

    log_info(&format!(
        "[wallpaper_picker] sobre el monitor principal, {}x{}",
        size.0, size.1
    ));

    // Escape y perder el foco la esconden desde GTK, sin pasar por la página:
    // se le avisa para que suelte la previsualización de video, que si no
    // seguiría decodificando en una superficie que nadie ve.
    let handle = app.clone();
    let spec = LayerSpec {
        on_hide: Some(Box::new(move || {
            if let Some(webview) = handle.get_webview_window(WALLPAPER_PICKER_LABEL) {
                let _ = webview.emit("window-hidden", ());
            }
        })),
        ..picker_spec()
    };

    spawn_layer_window(
        app,
        WALLPAPER_PICKER_LABEL,
        "index.html#/apps/wallpaper-picker",
        &gdk_monitor,
        size,
        spec,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubre_la_pantalla_entera_sin_correr_las_ventanas() {
        let spec = picker_spec();
        assert_eq!(spec.anchors, (true, true, true, true));
        assert_eq!(spec.exclusive_zone, Some(-1));
        assert_eq!(spec.layer, Layer::Overlay);
    }

    #[test]
    fn escape_y_perder_el_foco_la_cierran() {
        let spec = picker_spec();
        assert!(spec.dismiss_on_unfocus);
        assert_eq!(spec.keyboard, KeyboardMode::OnDemand);
        // Se arma al pedirla, así que nace visible.
        assert!(!spec.start_hidden);
    }
}
