use gtk_layer_shell::{KeyboardMode, Layer};
use tauri::AppHandle;

use crate::logger::log_info;
use crate::monitor_manager::{find_gdk_monitor, get_primary_monitor};
use crate::windows_apps::shell_layer::{spawn_layer_window, LayerSpec};

pub const CONTROL_CENTER_LABEL: &str = "control_center";

const WIDTH: f64 = 350.0;
/// Gap from the screen edges, and from the panel above.
const MARGIN: i32 = 10;
const PANEL_HEIGHT: i32 = 38;

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
    let height = monitor_height - (PANEL_HEIGHT + MARGIN * 2) as f64;

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
            anchors: (false, true, true, true),
            // Overlays reserve nothing: windows must not be pushed aside by a
            // panel that appears and disappears.
            exclusive_zone: Some(-1),
            margins: (0, MARGIN, PANEL_HEIGHT + MARGIN, MARGIN),
            // Needed for Escape to arrive and for losing focus to be noticed.
            keyboard: KeyboardMode::OnDemand,
            start_hidden: true,
            dismiss_on_unfocus: true,
        },
    )
}
