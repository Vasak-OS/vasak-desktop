pub mod wayland;
pub mod wayfire_ipc;

use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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

impl WindowManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        crate::logger::log_info("Inicializando Window Manager");

        match wayland::WaylandManager::new() {
            Ok(wayland_mgr) => {
                crate::logger::log_info("Backend Wayland/Wayfire inicializado correctamente");
                Ok(Self {
                    backend: Box::new(wayland_mgr),
                })
            }
            Err(e) => {
                crate::logger::log_error(&format!(
                    "No se pudo inicializar backend Wayland/Wayfire: {}",
                    e
                ));
                Err("No supported window system found".into())
            }
        }
    }

    pub fn get_window_list(&mut self) -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>> {
        self.backend.get_window_list()
    }

    pub fn toggle_window(&self, win_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.backend.toggle_window(win_id)
    }
}
