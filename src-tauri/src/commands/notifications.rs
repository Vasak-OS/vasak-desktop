use crate::notifications::{
    clear_all_notifications, get_notifications, remove_notification, send_system_notification,
};
use crate::structs::Notification;

#[tauri::command]
pub async fn send_notify(
    summary: String,
    body: Option<String>,
    urgency: Option<String>,
) -> Result<String, String> {
    send_system_notification(summary, body, urgency).await
}

#[tauri::command]
pub async fn clear_notifications() -> Result<u32, String> {
    clear_all_notifications().await
}

#[tauri::command]
pub async fn get_all_notifications() -> Result<Vec<Notification>, String> {
    get_notifications().await
}

#[tauri::command]
pub async fn delete_notification(id: u32) -> Result<bool, String> {
    remove_notification(id).await
}
