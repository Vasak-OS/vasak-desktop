pub mod sni_item;
pub mod sni_watcher;
pub mod dbus_menu;

use crate::structs::TrayManager;
use std::collections::HashMap;
use std::sync::Arc;
use crate::logger::log_error;
use tauri::{async_runtime::RwLock, AppHandle, Emitter};

pub fn create_tray_manager() -> TrayManager {
    Arc::new(RwLock::new(HashMap::new()))
}

pub async fn emit_tray_update(app_handle: &AppHandle) {
    if let Err(e) = app_handle.emit("tray-update", ()) {
        log_error(&format!("[Tray] Error emitiendo evento tray-update: {}", e));
    }
}
