use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::audio::get_volume;
use crate::applets::battery::{has_battery, get_battery_info as get_battery_info_internal};
use crate::notifications::get_notifications;
use crate::structs::{TrayManager, WMState};
use crate::logger::log_debug;

#[derive(Debug, Deserialize)]
pub struct BatchRequest {
    pub id: usize,
    pub command: String,
    pub args: Value,
}

#[derive(Debug, Serialize)]
pub struct BatchResponse {
    pub id: usize,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl BatchResponse {
    fn ok(id: usize, data: Value) -> Self {
        Self {
            id,
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn err(id: usize, error: String) -> Self {
        Self {
            id,
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// Dispatches a single command by name, using the app handle to access managed state.
async fn dispatch_command(app: &AppHandle, command: &str, _args: &Value) -> BatchResponse {
    // Placeholder id=0, caller will set the correct id
    match command {
        "get_windows" => {
            let state = app.state::<WMState>();
            match state
                .window_manager
                .read()
                .map_err(|e| e.to_string())
                .and_then(|wm| wm.get_window_list().map_err(|e| e.to_string()))
            {
                Ok(windows) => match serde_json::to_value(&windows) {
                    Ok(val) => BatchResponse::ok(0, val),
                    Err(e) => BatchResponse::err(0, e.to_string()),
                },
                Err(e) => BatchResponse::err(0, e),
            }
        }
        "get_tray_items" => {
            let tray_manager = app.state::<TrayManager>();
            let manager = tray_manager.read().await;
            let items: Vec<_> = manager.values().cloned().collect();
            match serde_json::to_value(&items) {
                Ok(val) => BatchResponse::ok(0, val),
                Err(e) => BatchResponse::err(0, e.to_string()),
            }
        }
        "get_all_notifications" => match get_notifications().await {
            Ok(notifications) => match serde_json::to_value(&notifications) {
                Ok(val) => BatchResponse::ok(0, val),
                Err(e) => BatchResponse::err(0, e.to_string()),
            },
            Err(e) => BatchResponse::err(0, e),
        },
        "get_audio_volume" => match get_volume() {
            Ok(volume_info) => match serde_json::to_value(&volume_info) {
                Ok(val) => BatchResponse::ok(0, val),
                Err(e) => BatchResponse::err(0, e.to_string()),
            },
            Err(e) => BatchResponse::err(0, e.to_string()),
        },
        "battery_exists" => {
            let exists = has_battery().await;
            BatchResponse::ok(0, Value::Bool(exists))
        }
        "get_battery_info" | "battery_fetch_info" => {
            let info = get_battery_info_internal().await;
            match serde_json::to_value(&info) {
                Ok(val) => BatchResponse::ok(0, val),
                Err(e) => BatchResponse::err(0, e.to_string()),
            }
        }
        _ => BatchResponse::err(0, format!("Unknown command: {}", command)),
    }
}

#[tauri::command]
pub async fn batch_invoke(
    app: AppHandle,
    requests: Vec<BatchRequest>,
) -> Vec<BatchResponse> {
    log_debug(&format!("batch_invoke: processing {} requests", requests.len()));

    let futures: Vec<_> = requests
        .iter()
        .map(|req| {
            let app_clone = app.clone();
            let command = req.command.clone();
            let args = req.args.clone();
            let id = req.id;
            async move {
                let mut response = dispatch_command(&app_clone, &command, &args).await;
                response.id = id;
                response
            }
        })
        .collect();

    futures_util::future::join_all(futures).await
}
