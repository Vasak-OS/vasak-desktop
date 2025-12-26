use crate::window_manager::WindowManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::async_runtime::RwLock;

pub struct WMState {
    pub(crate) window_manager: Arc<Mutex<WindowManager>>,
}

// Estructura para notificaciones reales
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub app_icon: String,
    pub timestamp: u64,
    pub seen: bool,
    pub urgency: NotificationUrgency,
    pub actions: Vec<String>,
    pub hints: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationUrgency {
    Low,
    Normal,
    Critical,
}

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

#[derive(Debug, Serialize, Clone)]
pub struct BrightnessInfo {
    pub current: u32,
    pub max: u32,
    pub min: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct VolumeInfo {
    pub current: i64,
    pub min: i64,
    pub max: i64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrayStatus {
    Active,
    Passive,
    NeedsAttention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrayCategory {
    ApplicationStatus,
    Communications,
    SystemServices,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album_art_url: Option<String>,
    pub player: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct BatteryInfo {
    pub has_battery: bool,
    pub percentage: f64,
    pub state: String,
    pub time_to_empty: Option<u64>,
    pub time_to_full: Option<u64>,
    pub is_present: bool,
    pub is_charging: bool,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub technology: Option<String>,
    pub energy: Option<f64>,
    pub energy_full: Option<f64>,
    pub energy_full_design: Option<f64>,
    pub voltage: Option<f64>,
    pub temperature: Option<f64>,
    pub serial: Option<String>,
}