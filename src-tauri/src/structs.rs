use crate::window_manager::WindowManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::async_runtime::RwLock;

/// Estado global del gestor de ventanas
pub struct WMState {
    pub(crate) window_manager: Arc<Mutex<WindowManager>>,
}

/// Representa una notificación del sistema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// ID único de la notificación
    pub id: u32,
    /// Nombre de la aplicación que envió la notificación
    pub app_name: String,
    /// Resumen o título de la notificación
    pub summary: String,
    /// Cuerpo o contenido de la notificación
    pub body: String,
    /// Icono de la aplicación
    pub app_icon: String,
    /// Timestamp de cuando se creó la notificación
    pub timestamp: u64,
    /// Indica si la notificación ha sido vista
    pub seen: bool,
    /// Nivel de urgencia de la notificación
    pub urgency: NotificationUrgency,
    /// Acciones disponibles para la notificación
    pub actions: Vec<String>,
    /// Hints adicionales de la notificación
    pub hints: HashMap<String, String>,
}

/// Nivel de urgencia de una notificación
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationUrgency {
    /// Urgencia baja
    Low,
    /// Urgencia normal
    Normal,
    /// Urgencia crítica
    Critical,
}

#[allow(dead_code)]
#[derive(Default)]
pub struct NotificationData {
    pub app_name: String,
    pub summary: String,
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppEntry {
    pub category: String,
    pub name: String,
    pub generic: String,
    pub description: String,
    pub icon: String,
    pub keywords: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryInfo {
    pub icon: String,
    pub description: String,
    pub apps: Vec<AppEntry>,
}

/// Información sobre el brillo del sistema
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct BrightnessInfo {
    /// Nivel de brillo actual (0-100)
    pub current: u32,
    /// Nivel máximo de brillo
    pub max: u32,
    /// Nivel mínimo de brillo
    pub min: u32,
}

/// Información sobre el volumen del sistema
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct VolumeInfo {
    /// Nivel de volumen actual (0-100)
    pub current: i64,
    /// Nivel mínimo de volumen
    pub min: i64,
    /// Nivel máximo de volumen
    pub max: i64,
    /// Indica si el audio está silenciado
    pub is_muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_default: bool,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayItem {
    pub id: String,
    pub service_name: String,
    pub icon_name: Option<String>,
    pub icon_data: Option<String>,
    pub title: Option<String>,
    pub tooltip: Option<String>,
    pub status: TrayStatus,
    pub category: TrayCategory,
    pub menu_path: Option<String>,
}

/// Estado de un elemento del system tray
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrayStatus {
    /// El elemento está activo
    Active,
    /// El elemento está pasivo
    Passive,
    /// El elemento necesita atención
    NeedsAttention,
}

/// Categoría de un elemento del system tray
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrayCategory {
    /// Estado de aplicación
    ApplicationStatus,
    /// Comunicaciones
    Communications,
    /// Servicios del sistema
    SystemServices,
    /// Hardware
    Hardware,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayMenu {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub visible: bool,
    #[serde(rename = "type")]
    pub menu_type: String,
    pub checked: Option<bool>,
    pub icon: Option<String>,
    pub children: Option<Vec<TrayMenu>>,
}

pub type TrayManager = Arc<RwLock<HashMap<String, TrayItem>>>;

/// Información sobre el reproductor de medios actual
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaInfo {
    /// Título de la pista actual
    pub title: Option<String>,
    /// Artista de la pista actual
    pub artist: Option<String>,
    /// URL del arte del álbum
    pub album_art_url: Option<String>,
    /// Nombre del reproductor
    pub player: Option<String>,
    /// Estado de reproducción (Playing, Paused, Stopped)
    pub status: Option<String>,
}

/// Información detallada sobre la batería del sistema
#[derive(Debug, Serialize, Clone)]
pub struct BatteryInfo {
    /// Indica si el sistema tiene batería
    pub has_battery: bool,
    /// Porcentaje de carga (0.0 - 100.0)
    pub percentage: f64,
    /// Estado de la batería (Charging, Discharging, Full, etc.)
    pub state: String,
    /// Tiempo estimado hasta vaciar (en segundos)
    pub time_to_empty: Option<u64>,
    /// Tiempo estimado hasta carga completa (en segundos)
    pub time_to_full: Option<u64>,
    /// Indica si la batería está presente
    pub is_present: bool,
    /// Indica si la batería está cargando
    pub is_charging: bool,
    /// Fabricante de la batería
    pub vendor: Option<String>,
    /// Modelo de la batería
    pub model: Option<String>,
    /// Tecnología de la batería
    pub technology: Option<String>,
    /// Energía actual (Wh)
    pub energy: Option<f64>,
    /// Energía máxima actual (Wh)
    pub energy_full: Option<f64>,
    /// Energía máxima de diseño (Wh)
    pub energy_full_design: Option<f64>,
    /// Voltaje actual (V)
    pub voltage: Option<f64>,
    /// Temperatura (°C)
    pub temperature: Option<f64>,
    /// Número de serie
    pub serial: Option<String>,
}