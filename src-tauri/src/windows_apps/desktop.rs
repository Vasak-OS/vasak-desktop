use gtk_layer_shell::Layer;
use tauri::AppHandle;

use crate::monitor_manager::{find_gdk_monitor, get_monitors, get_primary_monitor, label_for};
use crate::windows_apps::shell_layer::{spawn_layer_window, LayerSpec};

/// Creates the wallpaper/desktop surface on every connected monitor.
pub fn create_desktops(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let monitors = get_monitors(app).ok_or("No monitors found")?;
    let primary = get_primary_monitor(app).ok_or("No primary monitor found")?;

    for (index, monitor) in monitors.iter().enumerate() {
        let label = label_for("desktop", monitor, &primary, index);

        if let Err(error) = setup_desktop(app, &label, monitor) {
            log::error!("Desktop {} failed: {}", label, error);
        }
    }

    Ok(())
}

fn setup_desktop(
    app: &AppHandle,
    label: &str,
    monitor: &tauri::Monitor,
) -> Result<(), Box<dyn std::error::Error>> {
    let gdk_monitor =
        find_gdk_monitor(monitor).ok_or_else(|| format!("No GDK monitor for {}", label))?;

    // Tauri reports physical pixels; GDK and layer-shell work in logical ones.
    let scale = monitor.scale_factor();
    let size = monitor.size();
    let logical = (
        size.width as f64 / scale,
        size.height as f64 / scale,
    );

    spawn_layer_window(
        app,
        label,
        &format!("index.html#/desktop?monitor={}", label),
        &gdk_monitor,
        logical,
        LayerSpec {
            namespace: "vasak-desktop",
            layer: Layer::Background,
            anchors: (true, true, true, true),
            // -1 opts out of space reservation: the wallpaper must not push
            // windows around, and everything else draws on top of it.
            exclusive_zone: Some(-1),
            height_request: None,
        },
    )
}
