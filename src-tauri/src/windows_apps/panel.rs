use gtk_layer_shell::Layer;
use tauri::AppHandle;

use crate::logger::{log_error, log_info};
use crate::monitor_manager::{find_gdk_monitor, get_primary_monitor};
use crate::posicion_del_panel::{self, PosicionDelPanel};
use crate::windows_apps::shell_layer::{
    reubicar_layer_window, spawn_layer_window, Geometria, LayerSpec,
};

pub const PANEL_LABEL: &str = "panel";

/// Lo que mide la pantalla del panel, en píxeles lógicos.
///
/// Tauri informa en píxeles físicos y layer-shell trabaja en lógicos, así que
/// hay que dividir por la escala: en una pantalla HiDPI el panel pedía el doble
/// de ancho del que hay.
fn pantalla_del_panel(
    app: &AppHandle,
) -> Result<(gdk::Monitor, f64, f64), Box<dyn std::error::Error>> {
    let primary = get_primary_monitor(app).ok_or("No primary monitor found")?;
    let gdk_monitor =
        find_gdk_monitor(&primary).ok_or("No GDK monitor matching the primary monitor")?;

    let scale = primary.scale_factor();

    Ok((
        gdk_monitor,
        primary.size().width as f64 / scale,
        primary.size().height as f64 / scale,
    ))
}

/// Creates the panel, which lives only on the primary monitor by design: the
/// secondary screens get a desktop surface but no panel.
///
/// De qué lado queda lo dice `panel.position` en la configuración. Se lee acá y
/// no se guarda en ningún lado porque esta misma función es la que rehace el
/// panel al cambiar los monitores: leyéndolo cada vez, el panel reaparece
/// donde corresponde sin que el cambio de monitores tenga que saber nada de
/// esto.
pub fn create_panels(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let (gdk_monitor, ancho, alto) = pantalla_del_panel(app)?;
    let posicion = posicion_del_panel::leer();

    log_info(&format!("[panel] posición: {}", posicion.clave()));

    spawn_layer_window(
        app,
        PANEL_LABEL,
        "index.html#/panel",
        &gdk_monitor,
        posicion.tamano(ancho, alto),
        LayerSpec {
            namespace: "vasak-panel",
            layer: Layer::Top,
            anchors: posicion.anclas(),
            // Automatic: the panel reserves its strip so windows don't sit under it.
            exclusive_zone: None,
            ..Default::default()
        },
    )
}

/// Mueve el panel al lado que diga `posicion`, sin volver a crearlo.
///
/// Sigue en la misma pantalla: la superficie conserva el monitor al que se la
/// ancló, así que cambiar de lado no la manda a otro. Mover monitores es otra
/// cosa y la rehace entera.
///
/// Sólo desde el hilo principal de GTK. Ver [`reubicar_layer_window`].
pub fn reubicar_panel(app: &AppHandle, posicion: PosicionDelPanel) {
    let (ancho, alto) = match pantalla_del_panel(app) {
        Ok((_, ancho, alto)) => (ancho, alto),
        Err(error) => {
            log_error(&format!("[panel] no se pudo mover: {error}"));
            return;
        }
    };

    let movido = reubicar_layer_window(
        PANEL_LABEL,
        &Geometria {
            anchors: posicion.anclas(),
            size: posicion.tamano(ancho, alto),
            margins: (0, 0, 0, 0),
            exclusive_zone: None,
        },
    );

    if movido {
        log_info(&format!("[panel] movido a {}", posicion.clave()));
    } else {
        log_error("[panel] no está construido: no hay nada que mover");
    }
}
