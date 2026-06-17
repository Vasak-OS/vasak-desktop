use crate::logger::{log_info, log_warning};
use crate::window_manager::wayfire_ipc::get_wayfire_client;
use tokio::time::{sleep, Duration};

#[derive(Clone, Copy, Debug)]
pub enum WaylandLayerMode {
    Panel,
}

pub fn configure_wayland_layer(
    title: String,
    mode: WaylandLayerMode,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) {
    tauri::async_runtime::spawn(async move {
        if let Err(err) = apply_wayfire_geometry(&title, mode, x, y, width, height).await {
            log_warning(&format!("[wayland_layer] unable to apply Wayfire geometry for {title}: {err}"));
        }
    });
}

async fn apply_wayfire_geometry(
    title: &str,
    _mode: WaylandLayerMode,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let client = get_wayfire_client()
        .await
        .ok_or_else(|| "Wayfire IPC not available".to_string())?;

    let expected_title = title.to_lowercase();
    let mut found_view = None;

    for attempt in 0..30 {
        let views = client
            .list_views_typed()
            .await
            .map_err(|error| error.to_string())?;

        // For regular windows we only want the real toplevel view, not the desktop/background layer-shell.
        found_view = views.iter().find(|view| {
            let view_title = view.title.as_deref().unwrap_or_default().to_lowercase();
            let app_id = view.app_id.as_deref().unwrap_or_default().to_lowercase();
            let role = view.role.as_deref().unwrap_or_default().to_lowercase();

            if matches!(view.layer.as_deref(), Some("background") | Some("bottom")) {
                return false;
            }

            if !matches!(view.type_field.as_deref(), Some("toplevel")) {
                return false;
            }

            view_title == expected_title
                || view_title.contains(&expected_title)
                || app_id == expected_title
                || app_id.contains(&expected_title)
                || role == expected_title
                || role.contains(&expected_title)
        }).cloned();

        if found_view.is_some() {
            break;
        }

        // If this is the last attempt and we still haven't matched, log a sanitized view summary.
        if attempt == 29 {
            let summary: Vec<String> = views.iter().map(|v| {
                format!("id={} app_id={:?} role={:?} layer={:?} type={:?}",
                    v.id, v.app_id, v.role, v.layer, v.type_field)
            }).collect();
            log_warning(&format!("[wayland_layer] list-views (final attempt, {} views): {:?}", views.len(), summary));
        }

        sleep(Duration::from_millis(100)).await;
    }

    let Some(view) = found_view else {
        return Err(format!("view not found for title {title}"));
    };

    log_info(&format!("[wayland_layer] matched view id={} title={:?} app_id={:?} pid={:?} layer={:?}", view.id, view.title, view.app_id, view.pid, view.layer));

    let output_id = match client.list_outputs_typed().await {
        Ok(outputs) => {
            let matched_output = outputs.iter().find(|output| {
                let geometry = &output.geometry;
                let point_x = x as i64;
                let point_y = y as i64;

                point_x >= geometry.x
                    && point_x < geometry.x + geometry.width
                    && point_y >= geometry.y
                    && point_y < geometry.y + geometry.height
            });

            let selected = matched_output
                .map(|output| output.id as u64)
                .or_else(|| outputs.first().map(|output| output.id as u64));

            selected
        }
        Err(_) => None,
    };

    let view_id = view.id as u64;

    client.configure_view_coords(
        view_id,
        x as i64,
        y as i64,
        width as i64,
        height as i64,
        output_id,
    ).await.map_err(|e| format!("[wayland_layer] configure-view error for {title}: {e}"))?;

    // If the view was configured before map (common for panel), re-apply after it maps.
    if view.mapped != Some(true) {
        for _attempt in 0..30 {
            sleep(Duration::from_millis(100)).await;

            let Ok(all_views) = client.list_views_typed().await else {
                continue;
            };

            let Some(mapped_view) = all_views.into_iter().find(|v| v.id == view_id as i64) else {
                continue;
            };

            if mapped_view.mapped != Some(true) {
                continue;
            }

            client.configure_view_coords(
                view_id,
                x as i64,
                y as i64,
                width as i64,
                height as i64,
                output_id,
            ).await.map_err(|e| format!("[wayland_layer] reapply configure-view error for {title}: {e}"))?;

            break;
        }
    }

    client.set_sticky(view_id, true).await
        .map_err(|e| format!("[wayland_layer] set_sticky error for {title}: {e}"))?;
    client.set_always_on_top(view_id, true).await
        .map_err(|e| format!("[wayland_layer] set_always_on_top error for {title}: {e}"))?;

    Ok(())
}