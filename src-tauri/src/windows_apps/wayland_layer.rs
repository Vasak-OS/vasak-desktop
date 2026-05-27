use crate::logger::{log_info, log_warning};
use crate::window_manager::wayfire_ipc::get_wayfire_client;
use serde_json;
use tokio::time::{sleep, Duration};

#[derive(Clone, Copy, Debug)]
pub enum WaylandLayerMode {
    Panel,
    Desktop,
}

#[cfg(feature = "wayland")]
pub fn configure_wayland_layer(
    title: String,
    mode: WaylandLayerMode,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) {
    log_info(&format!("[wayland_layer] configure_wayland_layer called (mode={mode:?}, title={title})"));

    tauri::async_runtime::spawn(async move {
        if let Err(err) = apply_wayfire_geometry(&title, mode, x, y, width, height).await {
            log_warning(&format!("[wayland_layer] unable to apply Wayfire geometry for {title}: {err}"));
        }
    });
}

#[cfg(feature = "wayland")]
async fn apply_wayfire_geometry(
    title: &str,
    mode: WaylandLayerMode,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let client = get_wayfire_client()
        .await
        .ok_or_else(|| "Wayfire IPC not available".to_string())?;

    let expected_title = title.to_lowercase();
    let current_pid = std::process::id() as i64;
    let mut found_view = None;

    for attempt in 0..30 {
        let views = client
            .list_views_typed()
            .await
            .map_err(|error| error.to_string())?;

        // Try to match by a variety of heuristics, including type/layer fields.
        found_view = views.iter().find(|view| {
            let view_title = view.title.as_deref().unwrap_or_default().to_lowercase();
            let app_id = view.app_id.as_deref().unwrap_or_default().to_lowercase();
            let role = view.role.as_deref().unwrap_or_default().to_lowercase();
            let same_process = view.pid == Some(current_pid);
            let geom = view.geometry.as_ref().or(view.bbox.as_ref());
            let geom_matches = geom
                .map(|g| {
                    let width_close = (g.width - width as i64).abs() <= 16;
                    let height_close = (g.height - height as i64).abs() <= 16;
                    width_close && height_close
                })
                .unwrap_or(false);
            let mut matched = false;

            if view_title == expected_title
                || view_title.contains(&expected_title)
                || app_id == expected_title
                || app_id.contains(&expected_title)
                || role == expected_title
                || role.contains(&expected_title)
            {
                matched = true;
            }

            if !matched && (same_process && geom_matches) {
                matched = true;
            }

            // Also allow matching by known type/layer fields if present.
            if !matched {
                if let Some(type_field) = &view.type_field {
                    let tf = type_field.to_lowercase();
                    if tf.contains("panel") || tf.contains("desktop") || tf.contains("dock") {
                        matched = true;
                    }
                }
            }

            if !matched {
                if let Some(layer_field) = &view.layer {
                    let lf = layer_field.to_lowercase();
                    if lf == "top" || lf == "background" || lf == "overlay" {
                        matched = true;
                    }
                }
            }

            matched
        }).cloned();

        if found_view.is_some() {
            break;
        }

        // If this is the last attempt and we still haven't matched, log the full views for diagnosis.
        if attempt == 29 {
            if let Ok(serialized) = serde_json::to_string_pretty(&views) {
                log_warning(&format!("[wayland_layer] list-views (final attempt): {}", serialized));
            } else {
                log_warning("[wayland_layer] list-views: <failed to serialize views>");
            }
        }

        sleep(Duration::from_millis(100)).await;
    }

    let Some(view) = found_view else {
        return Err(format!("view not found for title {title}"));
    };

    log_info(&format!("[wayland_layer] matched view id={} title={:?} app_id={:?} pid={:?} layer={:?}", view.id, view.title, view.app_id, view.pid, view.layer));

    let output_id = match client.list_outputs_typed().await {
        Ok(outputs) => outputs
            .into_iter()
            .find(|output| {
                let geometry = &output.geometry;
                geometry.x == x as i64 && geometry.y == y as i64
            })
            .map(|output| output.id as u64),
        Err(_) => None,
    };

    let view_id = view.id as u64;

    match client.configure_view_coords(
        view_id,
        x as i64,
        y as i64,
        width as i64,
        height as i64,
        output_id,
    ).await {
        Ok(val) => log_info(&format!("[wayland_layer] configure-view response: {}", serde_json::to_string(&val).unwrap_or_else(|_| "<non-serializable>".into()))),
        Err(e) => log_warning(&format!("[wayland_layer] configure-view error: {}", e)),
    }

    // After configuration, fetch the view again and log its current properties for diagnosis.
    if let Ok(all_views) = client.list_views_typed().await {
        if let Some(updated) = all_views.into_iter().find(|v| v.id == view_id as i64) {
            if let Ok(serialized) = serde_json::to_string_pretty(&updated) {
                log_info(&format!("[wayland_layer] updated view after configure: {}", serialized));
            } else {
                log_warning("[wayland_layer] failed to serialize updated view");
            }
        } else {
            log_warning(&format!("[wayland_layer] view id={} not found after configure", view_id));
        }
    } else {
        log_warning("[wayland_layer] failed to list views after configure");
    }

    // If the view was configured before map (common for panel), re-apply after it maps.
    if view.mapped != Some(true) {
        for attempt in 0..30 {
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

            log_info(&format!(
                "[wayland_layer] view {} mapped=true on attempt {}, reapplying geometry",
                view_id,
                attempt + 1
            ));

            match client.configure_view_coords(
                view_id,
                x as i64,
                y as i64,
                width as i64,
                height as i64,
                output_id,
            ).await {
                Ok(val) => log_info(&format!("[wayland_layer] reapply configure-view: {}", serde_json::to_string(&val).unwrap_or_else(|_| "<non-serializable>".into()))),
                Err(e) => log_warning(&format!("[wayland_layer] reapply configure-view error: {}", e)),
            }

            break;
        }
    }

    match mode {
        WaylandLayerMode::Panel => {
            let _ = client.set_sticky(view_id, true).await;
            let _ = client.set_always_on_top(view_id, true).await;
        }
        WaylandLayerMode::Desktop => {
            let _ = client.set_sticky(view_id, true).await;
            let _ = client.send_to_back(view_id).await;
        }
    }

    Ok(())
}

#[cfg(not(feature = "wayland"))]
pub fn configure_wayland_layer(
    _title: String,
    _mode: WaylandLayerMode,
    _x: i32,
    _y: i32,
    _width: u32,
    _height: u32,
) {
    log_warning("[wayland_layer] wayland feature disabled; skipping Wayfire IPC setup");
}