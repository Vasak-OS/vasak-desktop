use super::{wayfire_ipc::{get_wayfire_client, View}, WindowInfo, WindowManagerBackend};
use std::sync::mpsc::Sender;

pub struct WaylandManager {
}

impl WaylandManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {})
    }

    fn is_shell_window(view: &View) -> bool {
        let title = view.title.as_deref().unwrap_or_default();
        let app_id = view.app_id.as_deref().unwrap_or_default();

        (app_id == "vasak-desktop")
            && (title == "Vasak Panel"
                || title == "Vasak Desktop"
                || title.starts_with("Vasak Desktop "))
    }

    fn view_to_window_info(view: &View) -> Option<WindowInfo> {
        if view.type_field.as_deref() != Some("toplevel") {
            return None;
        }

        if matches!(view.layer.as_deref(), Some("background") | Some("bottom")) {
            return None;
        }

        if Self::is_shell_window(view) {
            return None;
        }

        let title = view.title.clone().unwrap_or_default();
        let icon = view
            .app_id
            .clone()
            .or_else(|| view.role.clone())
            .unwrap_or_default();

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
        let windows = tauri::async_runtime::block_on(async {
            let client = get_wayfire_client().await.ok_or("Unable to connect to Wayfire IPC")?;
            let views = client.list_views_typed().await?;
            let mut windows: Vec<WindowInfo> = views.iter().filter_map(Self::view_to_window_info).collect();
            windows.sort_by(|left, right| left.id.cmp(&right.id));
            Result::<_, Box<dyn std::error::Error + Send + Sync>>::Ok(windows)
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

        tauri::async_runtime::block_on(async {
            let client = get_wayfire_client().await.ok_or("Unable to connect to Wayfire IPC")?;
            client.set_focus(view_id).await.map(|_| ())
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