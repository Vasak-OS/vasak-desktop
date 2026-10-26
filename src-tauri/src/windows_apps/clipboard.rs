use gtk_layer_shell::{KeyboardMode, Layer};
use tauri::AppHandle;

use crate::logger::log_info;
use crate::monitor_manager::{find_gdk_monitor, get_primary_monitor};
use crate::panel_position::{PANEL_THICKNESS, SCREEN_MARGIN};
use crate::windows_apps::shell_layer::{spawn_layer_window, LayerSpec};

pub const CLIPBOARD_LABEL: &str = "clipboard";

const WIDTH: f64 = 480.0;
const HEIGHT: f64 = 520.0;

/// Anclada abajo y centrada en horizontal. Como el centro de control, es una
/// superficie que aparece y desaparece: no reserva espacio y le pasa por encima
/// a lo que haya. Se aparta del borde de abajo lo suficiente para no quedar sobre
/// un panel que esté ahí (cuando el panel está en otro lado sobra un poco de
/// margen, que no molesta). El anclaje fino es de uiux.
pub fn create_clipboard_window(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let primary = get_primary_monitor(app).ok_or("No primary monitor found")?;
    let gdk_monitor =
        find_gdk_monitor(&primary).ok_or("No GDK monitor matching the primary monitor")?;

    let bottom = SCREEN_MARGIN + PANEL_THICKNESS;
    log_info(&format!(
        "[clipboard] anclado abajo y centrado, {WIDTH}x{HEIGHT}"
    ));

    spawn_layer_window(
        app,
        CLIPBOARD_LABEL,
        "index.html#/clipboard",
        &gdk_monitor,
        (WIDTH, HEIGHT),
        LayerSpec {
            namespace: "vasak-clipboard",
            // Sobre el panel, para que no lo recorte.
            layer: Layer::Overlay,
            // Sólo abajo: sin anclar izquierda ni derecha, el compositor la
            // centra en horizontal.
            anchors: (false, false, false, true),
            // No reserva espacio: las ventanas no se corren por ella.
            exclusive_zone: Some(-1),
            margins: (0, 0, 0, bottom),
            // Para que Escape llegue y para que perder el foco se note.
            keyboard: KeyboardMode::OnDemand,
            start_hidden: true,
            dismiss_on_unfocus: true,
            ..Default::default()
        },
    )
}
