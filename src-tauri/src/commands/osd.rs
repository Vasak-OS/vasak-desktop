use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::windows_apps::create_osd_window;

pub async fn show_osd_internal(
    icon: &str,
    value: f64,
    maximum: f64,
    label: &str,
    app: &AppHandle,
) -> Result<(), String> {
    let window = if let Some(w) = app.get_webview_window("osd_popup") {
        w
    } else {
        create_osd_window(app, icon, value, maximum, label)
            .await
            .map_err(|e| format!("Failed to create OSD window: {}", e))?
    };

    let _ = window.emit(
        "osd:show",
        json!({
            "icon": icon,
            "value": value,
            "maximum": maximum,
            "label": label,
        }),
    );

    let _ = window.show();
    let _ = window.set_focus();

    Ok(())
}

#[tauri::command]
pub async fn show_osd(
    icon: String,
    value: f64,
    maximum: f64,
    label: String,
    app: AppHandle,
) -> Result<(), String> {
    show_osd_internal(&icon, value, maximum, &label, &app).await
}
