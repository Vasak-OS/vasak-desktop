pub mod sni_watcher;
pub mod sni_item;
pub mod menu_parser;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

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

pub fn create_tray_manager() -> TrayManager {
    Arc::new(RwLock::new(HashMap::new()))
}

pub async fn emit_tray_update(app_handle: &AppHandle) {
    if let Err(e) = app_handle.emit("tray-update", ()) {
        eprintln!("[Tray] Error emitiendo evento tray-update: {}", e);
    }
}
