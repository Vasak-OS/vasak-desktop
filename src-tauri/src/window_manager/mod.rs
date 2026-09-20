pub mod app_icon;
pub mod delta;
pub mod present;
pub mod wayfire_ipc;
pub mod wayland;

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

pub trait WindowManagerBackend: Send + Sync {
    fn get_window_list(&self) -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>>;
    fn setup_event_monitoring(&mut self, tx: Sender<()>) -> Result<(), Box<dyn std::error::Error>>;
    fn toggle_window(&self, win_id: &str) -> Result<(), Box<dyn std::error::Error>>;
    /// Trae la ventana al frente. Nunca la minimiza.
    fn present_window(&self, win_id: &str) -> Result<(), Box<dyn std::error::Error>>;
}

/// Qué hacer con una ventana para dejarla al frente.
///
/// Aparte de la llamada al compositor porque es la parte que se puede
/// equivocar, y equivocarla hace lo contrario de lo que se pidió.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParaPresentar {
    /// Estaba minimizada: sacarla y enfocarla.
    RestaurarYEnfocar,
    /// Estaba a la vista: alcanza con enfocarla.
    Enfocar,
}

/// Lo que hay que hacer para presentar una ventana.
///
/// **Nunca minimiza**, y ésa es toda la diferencia con `toggle_window`. El
/// botón del panel alterna: apretar el de la ventana que estás usando la
/// esconde, que es lo que uno espera de un botón de la barra de tareas. Pero
/// desde el lanzador no: elegir una ventana en una lista de resultados no puede
/// esconderla, y `toggle_window` lo haría — el compositor puede seguir teniendo
/// la ventana como `activated` mientras el lanzador se queda con el teclado por
/// ser una superficie de capa, así que caería justo en la rama que minimiza.
pub fn para_presentar(minimizada: bool) -> ParaPresentar {
    if minimizada {
        ParaPresentar::RestaurarYEnfocar
    } else {
        ParaPresentar::Enfocar
    }
}

pub struct WindowManager {
    pub backend: Box<dyn WindowManagerBackend>,
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

    pub fn get_window_list(&self) -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>> {
        self.backend.get_window_list()
    }

    pub fn toggle_window(&self, win_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.backend.toggle_window(win_id)
    }

    pub fn present_window(&self, win_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.backend.present_window(win_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentar_nunca_minimiza() {
        // Es la diferencia con `toggle_window`, y la que importa: elegir una
        // ventana en una lista de resultados no puede esconderla.
        assert_eq!(para_presentar(false), ParaPresentar::Enfocar);
    }

    #[test]
    fn una_ventana_minimizada_se_restaura_antes_de_enfocarla() {
        // Enfocar una minimizada sin sacarla no la muestra: queda enfocada y
        // escondida, que para quien la eligió es que no pasó nada.
        assert_eq!(para_presentar(true), ParaPresentar::RestaurarYEnfocar);
    }
}
