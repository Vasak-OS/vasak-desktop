use gtk_layer_shell::{KeyboardMode, Layer};
use tauri::AppHandle;

use crate::logger::{log_error, log_info};
use crate::monitor_manager::{find_gdk_monitor, get_primary_monitor};
use crate::posicion_del_panel::{self, PosicionDelPanel, GROSOR_DEL_PANEL};
use crate::windows_apps::shell_layer::{
    reubicar_layer_window, spawn_layer_window, Geometria, LayerSpec,
};

pub const CONTROL_CENTER_LABEL: &str = "control_center";

const WIDTH: f64 = 350.0;
/// Gap from the screen edges, and from the panel.
const MARGIN: i32 = 10;

/// El borde derecho, de arriba abajo. No cambia con la posición del panel: lo
/// que cambia es cuánto se aparta.
const ANCLAS: (bool, bool, bool, bool) = (false, true, true, true);

/// Cuánto se aparta el centro de control de cada borde, y cuánto mide de alto.
///
/// El centro de control **no reserva espacio** —es una ventana que aparece y
/// desaparece, y las ventanas no se tienen que correr por eso—, y por lo mismo
/// tampoco respeta el espacio que reserva el panel: le pasa por encima. Así que
/// se aparta a mano del lado donde esté el panel.
///
/// Con el panel a la izquierda no hay nada que esquivar: el centro de control
/// vive pegado al borde derecho.
fn margenes_y_alto(
    posicion: PosicionDelPanel,
    alto_del_monitor: f64,
) -> ((i32, i32, i32, i32), f64) {
    let arriba = MARGIN
        + if posicion == PosicionDelPanel::Arriba {
            GROSOR_DEL_PANEL
        } else {
            0
        };
    let abajo = MARGIN
        + if posicion == PosicionDelPanel::Abajo {
            GROSOR_DEL_PANEL
        } else {
            0
        };
    let derecha = MARGIN
        + if posicion == PosicionDelPanel::Derecha {
            GROSOR_DEL_PANEL
        } else {
            0
        };

    (
        (0, derecha, arriba, abajo),
        alto_del_monitor - (arriba + abajo) as f64,
    )
}

/// Creates the control centre, anchored to the right edge of the primary
/// monitor.
///
/// It used to be an ordinary window placed with `set_position`, which does
/// nothing on Wayland — a client cannot decide where it sits, so the compositor
/// put it in the middle of the screen and a follow-up call to Wayfire's IPC
/// tried to drag it into place afterwards. Anchoring it as a layer surface is
/// how a shell component is meant to say where it belongs, and it works without
/// asking the compositor for a favour.
pub fn create_control_center_window(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let primary = get_primary_monitor(app).ok_or("No primary monitor found")?;
    let gdk_monitor =
        find_gdk_monitor(&primary).ok_or("No GDK monitor matching the primary monitor")?;

    let scale = primary.scale_factor();
    let monitor_height = primary.size().height as f64 / scale;
    let (margins, height) = margenes_y_alto(posicion_del_panel::leer(), monitor_height);

    log_info(&format!(
        "[control_center] anclado a la derecha, {}x{}",
        WIDTH, height
    ));

    spawn_layer_window(
        app,
        CONTROL_CENTER_LABEL,
        "index.html#/control_center",
        &gdk_monitor,
        (WIDTH, height),
        LayerSpec {
            namespace: "vasak-control-center",
            // Above the panel, so it is not clipped by it.
            layer: Layer::Overlay,
            // Right edge, spanning between the panel and the bottom.
            anchors: ANCLAS,
            // Overlays reserve nothing: windows must not be pushed aside by a
            // panel that appears and disappears.
            exclusive_zone: Some(-1),
            margins,
            // Needed for Escape to arrive and for losing focus to be noticed.
            keyboard: KeyboardMode::OnDemand,
            start_hidden: true,
            dismiss_on_unfocus: true,
        },
    )
}

/// Aparta el centro de control del panel, esté donde esté ahora.
///
/// Sin esto, mover el panel abajo le deja el centro de control encima de la
/// barra: no reserva espacio y se dibuja en la capa de arriba, así que tapa los
/// iconos en lugar de acomodarse.
///
/// Sólo desde el hilo principal de GTK. Ver [`reubicar_layer_window`].
pub fn reubicar_control_center(app: &AppHandle, posicion: PosicionDelPanel) {
    let Some(primary) = get_primary_monitor(app) else {
        log_error("[control_center] sin monitor primario: no se pudo acomodar");
        return;
    };

    let monitor_height = primary.size().height as f64 / primary.scale_factor();
    let (margins, height) = margenes_y_alto(posicion, monitor_height);

    if !reubicar_layer_window(
        CONTROL_CENTER_LABEL,
        &Geometria {
            anchors: ANCLAS,
            size: (WIDTH, height),
            margins,
            exclusive_zone: Some(-1),
        },
    ) {
        log_error("[control_center] no está construido: no hay nada que acomodar");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALTO: f64 = 1080.0;

    #[test]
    fn con_el_panel_arriba_el_centro_de_control_arranca_abajo_del_panel() {
        let (margenes, alto) = margenes_y_alto(PosicionDelPanel::Arriba, ALTO);

        assert_eq!(margenes, (0, MARGIN, MARGIN + GROSOR_DEL_PANEL, MARGIN));
        assert_eq!(alto, ALTO - (MARGIN * 2 + GROSOR_DEL_PANEL) as f64);
    }

    #[test]
    fn con_el_panel_abajo_el_hueco_queda_del_otro_lado() {
        // Es el caso que se rompía si esto no mirara la posición: el centro de
        // control quedaba con el hueco arriba —donde ya no hay panel— y tapando
        // la barra de abajo.
        let (margenes, alto) = margenes_y_alto(PosicionDelPanel::Abajo, ALTO);

        assert_eq!(margenes, (0, MARGIN, MARGIN, MARGIN + GROSOR_DEL_PANEL));
        assert_eq!(alto, ALTO - (MARGIN * 2 + GROSOR_DEL_PANEL) as f64);
    }

    #[test]
    fn con_el_panel_a_la_derecha_el_centro_de_control_se_corre_hacia_adentro() {
        // Comparten el borde derecho: sin apartarse, el centro de control se
        // dibuja encima de la barra.
        let (margenes, alto) = margenes_y_alto(PosicionDelPanel::Derecha, ALTO);

        assert_eq!(margenes, (0, MARGIN + GROSOR_DEL_PANEL, MARGIN, MARGIN));
        // Y de alto gana lo que ya no le saca el panel.
        assert_eq!(alto, ALTO - (MARGIN * 2) as f64);
    }

    #[test]
    fn con_el_panel_a_la_izquierda_no_hay_nada_que_esquivar() {
        let (margenes, alto) = margenes_y_alto(PosicionDelPanel::Izquierda, ALTO);

        assert_eq!(margenes, (0, MARGIN, MARGIN, MARGIN));
        assert_eq!(alto, ALTO - (MARGIN * 2) as f64);
    }

    #[test]
    fn el_centro_de_control_nunca_sale_de_la_pantalla() {
        // En una pantalla chica —o partida— los márgenes no pueden dar un alto
        // negativo, que es una superficie que el compositor no sabe dibujar.
        for posicion in [
            PosicionDelPanel::Arriba,
            PosicionDelPanel::Abajo,
            PosicionDelPanel::Izquierda,
            PosicionDelPanel::Derecha,
        ] {
            let (_, alto) = margenes_y_alto(posicion, 600.0);
            assert!(alto > 0.0, "{:?}", posicion);
        }
    }
}
