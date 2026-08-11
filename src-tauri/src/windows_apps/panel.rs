use gtk_layer_shell::Layer;
use tauri::AppHandle;

use crate::monitor_manager::{find_gdk_monitor, get_primary_monitor};
use crate::windows_apps::shell_layer::{spawn_layer_window, LayerSpec};

const PANEL_HEIGHT: i32 = 38;

/// Creates the panel, which lives only on the primary monitor by design: the
/// secondary screens get a desktop surface but no panel.
pub fn create_panels(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let primary = get_primary_monitor(app).ok_or("No primary monitor found")?;

    let gdk_monitor =
        find_gdk_monitor(&primary).ok_or("No GDK monitor matching the primary monitor")?;

    let scale = primary.scale_factor();
    let logical_width = primary.size().width as f64 / scale;

    spawn_layer_window(
        app,
        "panel",
        "index.html#/panel",
        &gdk_monitor,
        (logical_width, PANEL_HEIGHT as f64),
        LayerSpec {
            namespace: "vasak-panel",
            layer: Layer::Top,
            anchors: (true, true, true, false),
            // Automatic: the panel reserves its strip so windows don't sit under it.
            exclusive_zone: None,
            height_request: Some(PANEL_HEIGHT),
            ..Default::default()
        },
    )
}
