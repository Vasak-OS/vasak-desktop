use gtk_layer_shell::Layer;
use tauri::{AppHandle, Emitter};

use crate::logger::{log_error, log_info};
use crate::monitor_manager::{find_gdk_monitor, get_primary_monitor};
use crate::panel_position::{self, PanelPosition};
use crate::windows_apps::shell_layer::{
    relocate_layer_window, spawn_layer_window, Geometry, LayerSpec,
};

pub const PANEL_LABEL: &str = "panel";

/// Cuánto espacio reserva la superficie del panel.
///
/// Con auto-ocultar (`panel.autohide`) la zona exclusiva es cero: las ventanas
/// maximizadas ocupan también la franja del panel, y es la interfaz la que lo
/// esconde y lo revela al pasar el cursor por el borde. Sin él, `None` deja que
/// `gtk-layer-shell` reserve la franja automáticamente, como siempre.
fn panel_exclusive_zone() -> Option<i32> {
    if crate::panel_autohide::read() {
        Some(0)
    } else {
        None
    }
}

/// Lo que mide la pantalla del panel, en píxeles lógicos.
///
/// Tauri informa en píxeles físicos y layer-shell trabaja en lógicos, así que
/// hay que dividir por la escala: en una pantalla HiDPI el panel pedía el doble
/// de ancho del que hay.
pub fn panel_screen(
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
    let (gdk_monitor, width, height) = panel_screen(app)?;
    let position = panel_position::read();

    log_info(&format!("[panel] posición: {}", position.key()));

    // El puntero entrando y saliendo de la superficie, para el auto-ocultar: la
    // página no recibe la salida del puntero en una superficie de capa, así que
    // se la toma del `leave-notify` de GTK y se le avisa a la página por evento.
    // Se emiten siempre; la interfaz sólo les hace caso con el auto-ocultar puesto.
    let enter_app = app.clone();
    let leave_app = app.clone();

    spawn_layer_window(
        app,
        PANEL_LABEL,
        "index.html#/panel",
        &gdk_monitor,
        position.size(width, height),
        LayerSpec {
            namespace: "vasak-panel",
            layer: Layer::Top,
            anchors: position.anchors(),
            // Reserva su franja, salvo con auto-ocultar: ahí la zona es cero y las
            // ventanas ocupan también lo del panel.
            exclusive_zone: panel_exclusive_zone(),
            on_pointer_enter: Some(Box::new(move || {
                let _ = enter_app.emit("panel-pointer-entered", ());
            })),
            on_pointer_leave: Some(Box::new(move || {
                let _ = leave_app.emit("panel-pointer-left", ());
            })),
            ..Default::default()
        },
    )
}

/// Mueve el panel al lado que diga `position`, sin volver a crearlo.
///
/// Sigue en la misma pantalla: la superficie conserva el monitor al que se la
/// ancló, así que cambiar de lado no la manda a otro. Mover monitores es otra
/// cosa y la rehace entera.
///
/// Sólo desde el hilo principal de GTK. Ver [`relocate_layer_window`].
pub fn relocate_panel(app: &AppHandle, position: PanelPosition) {
    let (width, height) = match panel_screen(app) {
        Ok((_, width, height)) => (width, height),
        Err(error) => {
            log_error(&format!("[panel] no se pudo mover: {error}"));
            return;
        }
    };

    let moved = relocate_layer_window(
        PANEL_LABEL,
        &Geometry {
            anchors: position.anchors(),
            size: position.size(width, height),
            margins: (0, 0, 0, 0),
            // Se vuelve a leer en cada acomodo: prender o apagar el auto-ocultar
            // no cambia de lado el panel, pero sí cuánto reserva, y `relocate` es
            // lo que corre cuando la configuración cambia.
            exclusive_zone: panel_exclusive_zone(),
        },
    );

    if moved {
        log_info(&format!("[panel] moved a {}", position.key()));
    } else {
        log_error("[panel] no está construido: no hay nada que mover");
    }
}
