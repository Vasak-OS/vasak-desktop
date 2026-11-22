//! La superficie de chat de Lince (`vasak-assistant`): una superposición de
//! capa residente, anclada al borde derecho como el centro de control, donde se
//! habla con el asistente.
//!
//! Es casi igual al centro de control, con dos diferencias:
//!
//! - **No se cierra al perder foco** (`dismiss_on_unfocus: false`): una
//!   conversación no debe cerrarse mientras se escribe. Se cierra por el botón,
//!   por Escape y por el toggle, todos desde la página, que llama a
//!   `hide_assistant` (ver `commands/assistant.rs`).
//! - Es más ancha, para que entre la conversación.
//!
//! Hereda el matado/recarga del proceso de WebKit (`surface_lifecycle`): se
//! construye escondida, se le programa matar el proceso a los pocos segundos de
//! esconderse y se recarga al reabrirla. Es seguro porque el estado —la
//! conversación— no vive en el WebView sino en el servicio, atado a la conexión
//! del bus (ver `assistant.rs`); al recargar, la página pide `History` y lo
//! reconstruye.

use gtk_layer_shell::{KeyboardMode, Layer};
use tauri::AppHandle;

use crate::logger::{log_error, log_info};
use crate::monitor_manager::{find_gdk_monitor, get_primary_monitor};
use crate::panel_position::{self, PanelPosition};
use crate::windows_apps::control_center::margins_and_height;
use crate::windows_apps::shell_layer::{
    relocate_layer_window, spawn_layer_window, Geometry, LayerSpec,
};

pub const ASSISTANT_LABEL: &str = "assistant";

/// Ancho cómodo para una conversación. El alto lo da el panel, como el centro
/// de control.
const WIDTH: f64 = 420.0;

/// El borde derecho, de arriba abajo. Igual que el centro de control: lo que
/// cambia con la posición del panel es cuánto se aparta, no el borde.
const ANCHORS: (bool, bool, bool, bool) = (false, true, true, true);

/// Crea la superficie del asistente, anclada a la derecha del monitor primario.
///
/// Se construye escondida en el hilo principal de GTK (desde `setup()`), como
/// el centro de control: crearla después desde una tarea de Tokio tocaría GTK
/// desde otro hilo, que no es seguro.
pub fn create_assistant_window(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let primary = get_primary_monitor(app).ok_or("No primary monitor found")?;
    let gdk_monitor =
        find_gdk_monitor(&primary).ok_or("No GDK monitor matching the primary monitor")?;

    let scale = primary.scale_factor();
    let monitor_height = primary.size().height as f64 / scale;
    let (margins, height) = margins_and_height(panel_position::read(), monitor_height);

    log_info(&format!(
        "[assistant] anclado a la derecha, {}x{}",
        WIDTH, height
    ));

    let result = spawn_layer_window(
        app,
        ASSISTANT_LABEL,
        "index.html#/assistant",
        &gdk_monitor,
        (WIDTH, height),
        LayerSpec {
            namespace: "vasak-assistant-chat",
            // Encima del panel, para que no lo recorte.
            layer: Layer::Overlay,
            anchors: ANCHORS,
            // No reserva espacio: aparece y desaparece, no corre a las ventanas.
            exclusive_zone: Some(-1),
            margins,
            // Para que Escape y las teclas lleguen a la conversación.
            keyboard: KeyboardMode::OnDemand,
            start_hidden: true,
            // A diferencia del centro de control: una conversación no se cierra
            // sola al perder el foco. La cierra la página (botón/Escape/toggle).
            dismiss_on_unfocus: false,
            // Al esconderse se programa matar su WebKitWebProcess
            // (vasak-desktop#307). Seguro porque la conversación vive en el
            // servicio, no en la página.
            on_hide: {
                let app = app.clone();
                Some(Box::new(move || {
                    crate::commands::surface_lifecycle::arm_terminate_after_hide(
                        &app,
                        ASSISTANT_LABEL,
                    )
                }))
            },
            // Su propio WebKitWebProcess: lo matamos al esconderla, y no debe
            // arrastrar a las demás superficies co-alojadas (#309).
            isolated_process: true,
            ..Default::default()
        },
    );
    // Nace escondida: si nadie la abre, su proceso no vive toda la sesión. La
    // primera apertura cancela la muerte o —si ya corrió— recarga para revivirlo.
    if result.is_ok() {
        crate::commands::surface_lifecycle::arm_terminate_after_hide(app, ASSISTANT_LABEL);
    }
    result
}

/// Aparta la superficie del panel cuando este cambia de lado, sin recrearla.
///
/// Mismo motivo que el centro de control: no reserva espacio y se dibuja en la
/// capa de arriba, así que sin apartarse a mano quedaría encima de la barra.
/// Sólo desde el hilo principal de GTK.
pub fn relocate_assistant(app: &AppHandle, position: PanelPosition) {
    let Some(primary) = get_primary_monitor(app) else {
        log_error("[assistant] sin monitor primario: no se pudo acomodar");
        return;
    };

    let monitor_height = primary.size().height as f64 / primary.scale_factor();
    let (margins, height) = margins_and_height(position, monitor_height);

    if !relocate_layer_window(
        ASSISTANT_LABEL,
        &Geometry {
            anchors: ANCHORS,
            size: (WIDTH, height),
            margins,
            exclusive_zone: Some(-1),
        },
    ) {
        log_error("[assistant] no está construido: no hay nada que acomodar");
    }
}
