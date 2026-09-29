use gtk_layer_shell::{KeyboardMode, Layer};
use tauri::AppHandle;

use crate::logger::{log_error, log_info};
use crate::monitor_manager::{find_gdk_monitor, get_primary_monitor};
use crate::panel_position::{self, PanelPosition, PANEL_THICKNESS, SCREEN_MARGIN};
use crate::windows_apps::shell_layer::{
    relocate_layer_window, spawn_layer_window, Geometry, LayerSpec,
};

pub const CONTROL_CENTER_LABEL: &str = "control_center";

const WIDTH: f64 = 350.0;
/// Gap from the screen edges, and from the panel.
const MARGIN: i32 = SCREEN_MARGIN;

/// El borde derecho, de arriba abajo. No cambia con la posición del panel: lo
/// que cambia es cuánto se aparta.
const ANCHORS: (bool, bool, bool, bool) = (false, true, true, true);

/// Cuánto se aparta el centro de control de cada borde, y cuánto mide de alto.
///
/// El centro de control **no reserva espacio** —es una ventana que aparece y
/// desaparece, y las ventanas no se tienen que correr por eso—, y por lo mismo
/// tampoco respeta el espacio que reserva el panel: le pasa por encima. Así que
/// se aparta a mano del lado donde esté el panel.
///
/// Con el panel a la izquierda no hay nada que esquivar: el centro de control
/// vive pegado al borde derecho.
fn margins_and_height(position: PanelPosition, monitor_height: f64) -> ((i32, i32, i32, i32), f64) {
    let clearance =
        |side: PanelPosition| MARGIN + if position == side { PANEL_THICKNESS } else { 0 };
    let top = clearance(PanelPosition::Top);
    let bottom = clearance(PanelPosition::Bottom);
    let right = clearance(PanelPosition::Right);

    (
        (0, right, top, bottom),
        monitor_height - (top + bottom) as f64,
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
    let (margins, height) = margins_and_height(panel_position::read(), monitor_height);

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
            anchors: ANCHORS,
            // Overlays reserve nothing: windows must not be pushed aside by a
            // panel that appears and disappears.
            exclusive_zone: Some(-1),
            margins,
            // Needed for Escape to arrive and for losing focus to be noticed.
            keyboard: KeyboardMode::OnDemand,
            start_hidden: true,
            dismiss_on_unfocus: true,
            ..Default::default()
        },
    )
}

/// Aparta el centro de control del panel, esté donde esté ahora.
///
/// Sin esto, mover el panel abajo le deja el centro de control encima de la
/// barra: no reserva espacio y se dibuja en la capa de arriba, así que tapa los
/// iconos en lugar de acomodarse.
///
/// Sólo desde el hilo principal de GTK. Ver [`relocate_layer_window`].
pub fn relocate_control_center(app: &AppHandle, position: PanelPosition) {
    let Some(primary) = get_primary_monitor(app) else {
        log_error("[control_center] sin monitor primario: no se pudo acomodar");
        return;
    };

    let monitor_height = primary.size().height as f64 / primary.scale_factor();
    let (margins, height) = margins_and_height(position, monitor_height);

    if !relocate_layer_window(
        CONTROL_CENTER_LABEL,
        &Geometry {
            anchors: ANCHORS,
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

    const HEIGHT: f64 = 1080.0;

    #[test]
    fn con_el_panel_arriba_el_centro_de_control_arranca_abajo_del_panel() {
        let (margins, height) = margins_and_height(PanelPosition::Top, HEIGHT);

        assert_eq!(margins, (0, MARGIN, MARGIN + PANEL_THICKNESS, MARGIN));
        assert_eq!(height, HEIGHT - (MARGIN * 2 + PANEL_THICKNESS) as f64);
    }

    #[test]
    fn con_el_panel_abajo_el_hueco_queda_del_otro_lado() {
        // Es el caso que se rompía si esto no mirara la posición: el centro de
        // control quedaba con el hueco arriba —donde ya no hay panel— y tapando
        // la barra de abajo.
        let (margins, height) = margins_and_height(PanelPosition::Bottom, HEIGHT);

        assert_eq!(margins, (0, MARGIN, MARGIN, MARGIN + PANEL_THICKNESS));
        assert_eq!(height, HEIGHT - (MARGIN * 2 + PANEL_THICKNESS) as f64);
    }

    #[test]
    fn con_el_panel_a_la_derecha_el_centro_de_control_se_corre_hacia_adentro() {
        // Comparten el borde derecho: sin apartarse, el centro de control se
        // dibuja encima de la barra.
        let (margins, height) = margins_and_height(PanelPosition::Right, HEIGHT);

        assert_eq!(margins, (0, MARGIN + PANEL_THICKNESS, MARGIN, MARGIN));
        // Y de alto gana lo que ya no le saca el panel.
        assert_eq!(height, HEIGHT - (MARGIN * 2) as f64);
    }

    #[test]
    fn con_el_panel_a_la_izquierda_no_hay_nada_que_esquivar() {
        let (margins, height) = margins_and_height(PanelPosition::Left, HEIGHT);

        assert_eq!(margins, (0, MARGIN, MARGIN, MARGIN));
        assert_eq!(height, HEIGHT - (MARGIN * 2) as f64);
    }

    #[test]
    fn el_centro_de_control_nunca_sale_de_la_pantalla() {
        // En una pantalla chica —o partida— los márgenes no pueden dar un alto
        // negativo, que es una superficie que el compositor no sabe dibujar.
        for position in PanelPosition::ALL {
            let (_, height) = margins_and_height(position, 600.0);
            assert!(height > 0.0, "{:?}", position);
        }
    }
}
