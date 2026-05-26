#[cfg(feature = "wayland")]
pub mod wayland;
#[cfg(feature = "wayland")]
pub mod wayfire_ipc;
#[cfg(feature = "x11")]
pub mod x11;

use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub is_minimized: bool,
    pub icon: String,
    pub demands_attention: Option<bool>,
}

pub trait WindowManagerBackend {
    fn get_window_list(&mut self) -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>>;
    fn setup_event_monitoring(&mut self, tx: Sender<()>) -> Result<(), Box<dyn std::error::Error>>;
    fn toggle_window(&self, win_id: &str) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct WindowManager {
    pub backend: Box<dyn WindowManagerBackend + Send>,
}

fn is_wayfire_session() -> bool {
    let desktop_hint = std::env::var_os("XDG_CURRENT_DESKTOP")
        .or_else(|| std::env::var_os("XDG_SESSION_DESKTOP"))
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let session_type = std::env::var_os("XDG_SESSION_TYPE")
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let wayland_hint = std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var_os("WAYFIRE_IPC_SOCKET").is_some()
        || session_type == "wayland"
        || desktop_hint.contains("wayfire");

    let clearly_x11 = session_type == "x11"
        || (std::env::var_os("DISPLAY").is_some() && !wayland_hint);

    wayland_hint || !clearly_x11
}

impl WindowManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        crate::logger::log_info("Inicializando Window Manager");

        #[cfg(feature = "wayland")]
        {
            if is_wayfire_session() {
                match wayland::WaylandManager::new() {
                    Ok(wayland_mgr) => {
                        crate::logger::log_info("Backend Wayland/Wayfire inicializado correctamente");
                        return Ok(Self {
                            backend: Box::new(wayland_mgr),
                        });
                    }
                    Err(e) => {
                        crate::logger::log_warning(&format!(
                            "No se pudo inicializar backend Wayland/Wayfire: {}",
                            e
                        ));
                    }
                }
            } else {
                crate::logger::log_debug("Wayfire no detectado; saltando backend Wayland");
            }
        }

        #[cfg(feature = "x11")]
        if std::env::var("DISPLAY").is_ok() {
            crate::logger::log_info("Detectado X11, inicializando backend");
            return Ok(Self {
                backend: Box::new(x11::X11Manager::new()?),
            });
        }

        crate::logger::log_error("No se encontró sistema de ventanas soportado");
        Err("No supported window system found".into())
    }

    pub fn get_window_list(&mut self) -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>> {
        self.backend.get_window_list()
    }

    pub fn toggle_window(&self, win_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.backend.toggle_window(win_id)
    }
}
