#[cfg(feature = "wayland")]
pub mod wayland;
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

impl WindowManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        crate::logger::log_info("Inicializando Window Manager");
        #[cfg(feature = "wayland")]
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            crate::logger::log_info("Detectado Wayland, intentando inicializar backend");
            match wayland::WaylandManager::new() {
                Ok(wayland_mgr) => {
                    // Try to setup protocols to verify they work
                    let mut temp_mgr = wayland_mgr;
                    match temp_mgr.setup_protocol_bindings() {
                        Ok(_) => {
                            crate::logger::log_info("Backend Wayland inicializado correctamente");
                            return Ok(Self {
                                backend: Box::new(temp_mgr),
                            });
                        }
                        Err(e) => {
                            log::warn!("Wayland window management not available: {}", e);
                            crate::logger::log_warning(&format!("Gestión de ventanas Wayland no disponible: {}", e));
                            log::info!("Falling back to X11 window management...");
                            crate::logger::log_info("Recurriendo a gestión de ventanas X11...");
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to initialize Wayland manager: {}", e);
                    crate::logger::log_warning(&format!("Error al inicializar Wayland manager: {}", e));
                }
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
