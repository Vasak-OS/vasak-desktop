use super::{wayfire_ipc::{get_wayfire_client, View}, WindowInfo, WindowManagerBackend};
use std::sync::mpsc::Sender;

pub struct WaylandManager {
}

impl WaylandManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {})
    }

    fn normalize_icon_name(raw: &str) -> String {
        let candidate = raw.trim().to_lowercase();
        if candidate.is_empty() {
            return String::new();
        }

        let tail = candidate.rsplit('.').next().unwrap_or(&candidate);
        tail.replace(['_', ' '], "-")
    }

    fn is_shell_window(view: &View) -> bool {
        let title = view.title.as_deref().unwrap_or_default();
        let app_id = view.app_id.as_deref().unwrap_or_default();

        (app_id == "vasak-desktop")
            && (title == "Vasak Panel"
                || title == "Vasak Desktop"
                || title.starts_with("Vasak Desktop "))
    }

    fn is_layer_shell_window(view: &View) -> bool {
        if Self::is_shell_window(view) {
            return true;
        }

        view
            .type_field
            .as_deref()
            .map(|value| {
                matches!(
                    value.to_lowercase().as_str(),
                    "panel" | "desktop" | "dock" | "layer-shell" | "layershell"
                ) || value.to_lowercase().contains("layer-shell")
            })
            .unwrap_or(false)
    }

    fn view_to_window_info(view: &View) -> Option<WindowInfo> {
        if view.mapped == Some(false) {
            return None;
        }

        if matches!(view.layer.as_deref(), Some("background") | Some("bottom")) {
            return None;
        }

        if Self::is_layer_shell_window(view) {
            return None;
        }

        let title = view.title.clone().unwrap_or_default();
        let icon = view
            .app_id
            .as_deref()
            .map(Self::normalize_icon_name)
            .filter(|icon| !icon.is_empty())
            .or_else(|| {
                view.role
                    .as_deref()
                    .map(Self::normalize_icon_name)
                    .filter(|icon| !icon.is_empty())
            })
            .unwrap_or_else(|| "application-x-executable".to_string());

        if title.is_empty() && icon == "application-x-executable" {
            return None;
        }

        Some(WindowInfo {
            id: view.id.to_string(),
            title,
            is_minimized: view.minimized.unwrap_or(false),
            icon,
            demands_attention: view.sticky,
        })
    }
}

impl WindowManagerBackend for WaylandManager {
    fn get_window_list(&mut self) -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>> {
        let windows = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let client = get_wayfire_client().await.ok_or("Unable to connect to Wayfire IPC")?;
                let views = client.list_views_typed().await?;
                let mut windows: Vec<WindowInfo> = views.iter().filter_map(Self::view_to_window_info).collect();
                windows.sort_by(|left, right| left.id.cmp(&right.id));
                Result::<_, Box<dyn std::error::Error + Send + Sync>>::Ok(windows)
            })
        })
        .map_err(|error| -> Box<dyn std::error::Error> { error })?;

        Ok(windows)
    }

    fn setup_event_monitoring(&mut self, tx: Sender<()>) -> Result<(), Box<dyn std::error::Error>> {
        let client = tauri::async_runtime::block_on(async {
            get_wayfire_client()
                .await
                .ok_or("Unable to connect to Wayfire IPC")
        })?;
        let mut receiver = client.subscribe();

        let _ = tx.send(());

        tauri::async_runtime::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(_) => {
                        let _ = tx.send(());
                    }
                    Err(err) => {
                        log::warn!("Wayfire event stream closed: {}", err);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    fn toggle_window(&self, win_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let view_id = win_id.parse::<u64>().map_err(|error| format!("invalid Wayfire view id {win_id}: {error}"))?;
        let view_id_i64 = i64::try_from(view_id).map_err(|error| format!("Wayfire view id out of range {win_id}: {error}"))?;

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let client = get_wayfire_client().await.ok_or("Unable to connect to Wayfire IPC")?;
                let views = client.list_views_typed().await?;
                let view = views
                    .into_iter()
                    .find(|candidate| candidate.id == view_id_i64)
                    .ok_or_else(|| format!("Wayfire view not found: {view_id}"))?;

                if view.minimized.unwrap_or(false) {
                    client.set_minimized(view_id, false).await?;
                    client.set_focus(view_id).await.map(|_| ())
                } else if view.activated {
                    client.set_minimized(view_id, true).await.map(|_| ())
                } else {
                    client.set_focus(view_id).await.map(|_| ())
                }
            })
        })
        .map_err(|error| -> Box<dyn std::error::Error> { error })?;

        Ok(())
    }
}

impl Default for WaylandManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize WaylandManager")
    }
}