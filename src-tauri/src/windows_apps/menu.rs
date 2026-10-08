//! El menú de aplicaciones, anclado a su botón del panel.
//!
//! Antes era una ventana común de 900×620 que se centraba en el monitor con
//! `set_position`, y en Wayland eso no decide nada: el compositor la ponía
//! donde quería, lejos del botón. Ahora es un applet más —una fila en
//! [`APPLETS`](crate::windows_apps::anchored_applet::APPLETS)— y se abre por el
//! mismo camino que los demás: el mismo cálculo de lugar, los mismos bordes, la
//! misma animación que crece desde el botón.
//!
//! Lo que el menú tenía y sigue teniendo lo da ese camino: la superficie se
//! **esconde** y no se destruye (la página no se recarga), el panel se entera de
//! que está abierto por `applet-changed`, y al cambiar los monitores
//! `destroy_applets` la baja para que la próxima apertura la cree en el monitor
//! que corresponda.
//!
//! # Abrirlo sin botón
//!
//! Lo único propio es de dónde sale el ancla cuando lo que lo abre no es un
//! clic: `OpenMenu` por D-Bus, que es lo que dispara la tecla Super. Ahí no hay
//! rectángulo, pero el menú **tiene** un botón en el panel, así que se ancla a
//! ése. El panel informa dónde lo dibujó ([`remember_menu_button`]) al montarse
//! y cada vez que se reacomoda; si todavía no dijo nada, o lo dijo con el panel
//! de otro lado, se usa dónde lo dibuja el panel por diseño
//! ([`default_menu_button`]).

use std::sync::Mutex;

use tauri::AppHandle;

use crate::panel_position::{self, PanelPosition};
use crate::windows_apps::anchored_applet::{toggle_anchored_applet, AnchorRect};

/// El nombre del menú en la tabla de applets.
///
/// Como constante porque se nombra desde afuera de la tabla: el comando, las
/// pruebas de `monitor_manager` y el aviso de `applet-changed` que realza el
/// botón del panel.
pub const MENU_APPLET: &str = "menu";

/// El botón del menú, tal como lo midió el panel, y de qué lado estaba el panel
/// al medirlo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuButton {
    pub side: PanelPosition,
    pub rect: AnchorRect,
}

/// Lo último que informó el panel.
///
/// Un `Mutex` y no un `thread_local`: lo escribe un comando, que corre en el
/// hilo que le toque, y lo lee el conmutador desde otro.
static LAST_MENU_BUTTON: Mutex<Option<MenuButton>> = Mutex::new(None);

/// Guarda dónde dibujó el panel el botón del menú.
pub fn remember_menu_button(button: MenuButton) {
    if let Ok(mut last) = LAST_MENU_BUTTON.lock() {
        *last = Some(button);
    }
}

fn last_menu_button() -> Option<MenuButton> {
    LAST_MENU_BUTTON.lock().ok().and_then(|last| *last)
}

/// Dónde queda el botón del menú si el panel no dijo nada.
///
/// Es el primero de la barra: 4 píxeles de margen lateral de la barra, 1 de
/// borde y 12 de relleno (`BAR_CLASSES` en `panel-position.ts`), y el botón
/// mide 32 —el icono de 28 más su relleno—. A lo ancho del panel da igual
/// dónde esté: el cálculo sólo mira el centro a lo largo.
pub fn default_menu_button(side: PanelPosition) -> AnchorRect {
    const ALONG: f64 = 17.0;
    const ACROSS: f64 = 3.0;
    const SIZE: f64 = 32.0;

    if side.is_vertical() {
        AnchorRect {
            x: ACROSS,
            y: ALONG,
            width: SIZE,
            height: SIZE,
        }
    } else {
        AnchorRect {
            x: ALONG,
            y: ACROSS,
            width: SIZE,
            height: SIZE,
        }
    }
}

/// De qué botón cuelga el menú.
///
/// - El que se tocó, si lo abrió un clic.
/// - Si no, el que informó el panel, siempre que lo haya medido con el panel del
///   mismo lado que ahora: un rectángulo medido arriba no dice nada de dónde
///   está el botón con el panel a la izquierda.
/// - Si no, el lugar donde el panel lo dibuja por diseño.
pub fn resolve_menu_anchor(
    clicked: Option<AnchorRect>,
    side: PanelPosition,
    reported: Option<MenuButton>,
) -> AnchorRect {
    clicked
        .or_else(|| {
            reported
                .filter(|button| button.side == side)
                .map(|button| button.rect)
        })
        .unwrap_or_else(|| default_menu_button(side))
}

/// Abre o cierra el menú, colgado de su botón del panel.
///
/// `anchor` es el botón que se tocó; `None` cuando lo abre algo que no es el
/// panel (D-Bus, la tecla Super). Se puede llamar desde cualquier hilo.
pub fn toggle_menu(app: &AppHandle, anchor: Option<AnchorRect>) -> Result<(), String> {
    let side = panel_position::read();
    if let Some(rect) = anchor {
        // El clic también cuenta como informe: es la medida más fresca que hay.
        remember_menu_button(MenuButton { side, rect });
    }
    let anchor = resolve_menu_anchor(anchor, side, last_menu_button());
    // El tamaño con que abre el menú sale de `menu.displayMode` (vasak-desktop#210):
    // `normal` usa el del `AppletSpec`, `compact` uno chico, `full` uno grande
    // que el cálculo de ubicación acota al monitor (el overlay propio lo agrega
    // el backend, ver `menu_display::size_override`).
    let size = crate::menu_display::size_override(crate::menu_display::read());
    toggle_anchored_applet(app, MENU_APPLET, Some(anchor), size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panel_position::{PANEL_THICKNESS, SCREEN_MARGIN};
    use crate::windows_apps::anchored_applet::{applet_spec, place_applet, PANEL_GAP};

    const MONITOR: (f64, f64) = (1920.0, 1080.0);
    /// La pantalla chica de referencia: un portátil de 13 pulgadas.
    const LAPTOP: (f64, f64) = (1366.0, 768.0);
    const AWAY: f64 = (PANEL_THICKNESS + PANEL_GAP) as f64;
    const M: f64 = SCREEN_MARGIN as f64;

    fn menu_size() -> (f64, f64) {
        applet_spec(MENU_APPLET)
            .expect("el menú está en la tabla")
            .size
    }

    fn rect(x: f64, y: f64) -> AnchorRect {
        AnchorRect {
            x,
            y,
            width: 32.0,
            height: 32.0,
        }
    }

    /// Dónde se ve el menú —sin el margen de sombra— abierto desde `anchor`:
    /// (x, y, ancho, alto) en el monitor.
    fn visible(
        anchor: Option<AnchorRect>,
        side: PanelPosition,
        monitor: (f64, f64),
    ) -> (f64, f64, f64, f64) {
        let anchor = resolve_menu_anchor(anchor, side, None);
        place_applet(Some(&anchor), side, menu_size(), monitor).visible_rect(monitor)
    }

    #[test]
    fn el_menu_es_un_applet_de_la_tabla() {
        // Es lo que le da todo lo demás: el mismo cálculo, esconderse en vez de
        // destruirse y que `destroy_applets` lo baje al cambiar los monitores.
        let spec = applet_spec(MENU_APPLET).expect("el menú está en la tabla");
        assert_eq!(spec.label(), "applet_menu");
        assert_eq!(spec.route, "menu");
        // Cambia dónde se abre, no cómo se ve.
        assert_eq!(spec.size, (900.0, 620.0));
    }

    #[test]
    fn con_el_panel_arriba_cuelga_debajo_y_se_corre_hasta_el_margen() {
        // El botón está pegado al borde izquierdo y el menú mide 900: centrado
        // sobre el botón se saldría, así que se corre hasta el margen del
        // escritorio, debajo del panel.
        assert_eq!(
            visible(None, PanelPosition::Top, MONITOR),
            (M, AWAY, 900.0, 620.0)
        );
        let anchor = resolve_menu_anchor(None, PanelPosition::Top, None);
        let placement = place_applet(Some(&anchor), PanelPosition::Top, menu_size(), MONITOR);
        // La entrada sigue creciendo desde el botón, no desde el centro.
        assert_eq!(placement.origin, 33.0 - M);
        assert_eq!(placement.anchors, (true, false, true, false));
    }

    #[test]
    fn con_el_panel_abajo_sube_desde_el_boton() {
        assert_eq!(
            visible(None, PanelPosition::Bottom, MONITOR),
            (M, 1080.0 - AWAY - 620.0, 900.0, 620.0)
        );
    }

    #[test]
    fn con_el_panel_a_la_izquierda_sale_hacia_la_derecha() {
        assert_eq!(
            visible(None, PanelPosition::Left, MONITOR),
            (AWAY, M, 900.0, 620.0)
        );
    }

    #[test]
    fn con_el_panel_a_la_derecha_sale_hacia_la_izquierda() {
        assert_eq!(
            visible(None, PanelPosition::Right, MONITOR),
            (1920.0 - AWAY - 900.0, M, 900.0, 620.0)
        );
    }

    #[test]
    fn un_boton_pegado_al_otro_extremo_tampoco_lo_saca_del_monitor() {
        // Si el botón quedara al final de la barra —el reloj, la bandeja—.
        let (x, _, width, _) = visible(Some(rect(1880.0, 3.0)), PanelPosition::Top, MONITOR);
        assert_eq!(x + width, 1920.0 - M);

        let (_, y, _, height) = visible(Some(rect(3.0, 1040.0)), PanelPosition::Left, MONITOR);
        assert_eq!(y + height, 1080.0 - M);
    }

    #[test]
    fn un_boton_en_el_medio_del_panel_centra_el_menu_sobre_el() {
        let (x, _, width, _) = visible(Some(rect(944.0, 3.0)), PanelPosition::Top, MONITOR);
        assert_eq!(x + width / 2.0, 960.0);
    }

    #[test]
    fn en_un_portatil_de_1366_por_768_entra_entero_de_los_cuatro_lados() {
        for side in PanelPosition::ALL {
            let (x, y, width, height) = visible(None, side, LAPTOP);
            assert_eq!((width, height), (900.0, 620.0), "{side:?}");
            assert!(x >= M && x + width <= LAPTOP.0 - M, "{side:?}: {x}");
            assert!(y >= M && y + height <= LAPTOP.1 - M, "{side:?}: {y}");
        }
        // Con el panel abajo, el borde de arriba del menú queda a 102 del de la
        // pantalla: 768 − 46 − 620.
        assert_eq!(visible(None, PanelPosition::Bottom, LAPTOP).1, 102.0);
    }

    #[test]
    fn en_un_monitor_mas_chico_se_achica_para_entrar() {
        let small = (1024.0, 600.0);

        // Arriba o abajo, el alto se come el panel, la separación y el margen.
        let (_, _, width, height) = visible(None, PanelPosition::Top, small);
        assert_eq!((width, height), (900.0, 600.0 - AWAY - M));

        // A un costado, el alto se come dos márgenes y el ancho entra.
        let (_, _, width, height) = visible(None, PanelPosition::Left, small);
        assert_eq!((width, height), (900.0, 600.0 - 2.0 * M));

        // Y en uno de 800 de ancho, el ancho también.
        let (_, _, width, _) = visible(None, PanelPosition::Top, (800.0, 600.0));
        assert_eq!(width, 800.0 - 2.0 * M);
    }

    #[test]
    fn la_superficie_nunca_sale_del_monitor_con_el_margen_de_sombra() {
        for monitor in [MONITOR, LAPTOP, (1024.0, 600.0)] {
            for side in PanelPosition::ALL {
                let anchor = resolve_menu_anchor(None, side, None);
                let placement = place_applet(Some(&anchor), side, menu_size(), monitor);
                let (left, right, top, bottom) = placement.margins;
                assert!(
                    left >= 0 && right >= 0 && top >= 0 && bottom >= 0,
                    "{side:?} {monitor:?}"
                );
                let (x, y, width, height) = placement.visible_rect(monitor);
                let (il, ir, it, ib) = placement.inset;
                assert!(x - il as f64 >= 0.0 && x + width + ir as f64 <= monitor.0);
                assert!(y - it as f64 >= 0.0 && y + height + ib as f64 <= monitor.1);
            }
        }
    }

    #[test]
    fn sin_clic_se_ancla_al_boton_que_informo_el_panel() {
        // `OpenMenu` por D-Bus —la tecla Super— no trae rectángulo.
        let reported = MenuButton {
            side: PanelPosition::Top,
            rect: rect(120.0, 3.0),
        };
        assert_eq!(
            resolve_menu_anchor(None, PanelPosition::Top, Some(reported)),
            rect(120.0, 3.0)
        );
    }

    #[test]
    fn sin_clic_y_sin_informe_va_al_lugar_del_boton_y_no_al_centro() {
        for side in PanelPosition::ALL {
            let anchor = resolve_menu_anchor(None, side, None);
            assert_eq!(anchor, default_menu_button(side), "{side:?}");

            let placement = place_applet(Some(&anchor), side, menu_size(), MONITOR);
            let centered = place_applet(None, side, menu_size(), MONITOR);
            assert_ne!(placement.margins, centered.margins, "{side:?}");
        }
    }

    #[test]
    fn un_informe_con_el_panel_de_otro_lado_no_vale() {
        // El panel se mudó y todavía no volvió a medir: el rectángulo viejo es
        // de una barra horizontal, y usarlo con la vertical pondría el menú a la
        // altura de la `y` de antes.
        let stale = MenuButton {
            side: PanelPosition::Top,
            rect: rect(600.0, 3.0),
        };
        assert_eq!(
            resolve_menu_anchor(None, PanelPosition::Left, Some(stale)),
            default_menu_button(PanelPosition::Left)
        );
    }

    #[test]
    fn el_clic_gana_sobre_lo_informado() {
        let reported = MenuButton {
            side: PanelPosition::Top,
            rect: rect(120.0, 3.0),
        };
        assert_eq!(
            resolve_menu_anchor(Some(rect(17.0, 3.0)), PanelPosition::Top, Some(reported)),
            rect(17.0, 3.0)
        );
    }

    #[test]
    fn el_boton_por_omision_esta_al_principio_del_panel() {
        // A lo largo del panel: arriba y abajo en la `x`, a los costados en la `y`.
        assert_eq!(default_menu_button(PanelPosition::Top).x, 17.0);
        assert_eq!(default_menu_button(PanelPosition::Bottom).x, 17.0);
        assert_eq!(default_menu_button(PanelPosition::Left).y, 17.0);
        assert_eq!(default_menu_button(PanelPosition::Right).y, 17.0);
    }
}
