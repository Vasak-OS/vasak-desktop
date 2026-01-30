use crate::notifications::{
    clear_all_notifications, get_notifications, remove_notification, send_system_notification,
};
use crate::logger::{log_info, log_error};
use crate::structs::Notification;

#[tauri::command]
pub async fn send_notify(
    summary: String,
    body: Option<String>,
    urgency: Option<String>,
) -> Result<String, String> {
    log_info(&format!("Enviando notificación: {}", summary));
    send_system_notification(summary, body, urgency).await.map_err(|e| {
        log_error(&format!("Error al enviar notificación: {}", e));
        e
    })
}

#[tauri::command]
pub async fn clear_notifications() -> Result<u32, String> {
    log_info("Limpiando todas las notificaciones");
    let result = clear_all_notifications().await;
    if let Ok(count) = result {
        log_info(&format!("Se limpiaron {} notificaciones", count));
    }
    result
}

#[tauri::command]
pub async fn get_all_notifications() -> Result<Vec<Notification>, String> {
    get_notifications().await
}

#[tauri::command]
pub async fn delete_notification(id: u32) -> Result<bool, String> {
    log_info(&format!("Eliminando notificación: {}", id));
    remove_notification(id).await
}

#[tauri::command]
pub async fn invoke_notification_action(id: u32, action_key: String) -> Result<(), String> {
    log_info(&format!("Invocando acción '{}' en notificación {}", action_key, id));
    crate::notifications::invoke_action(id, action_key).await.map_err(|e| {
        log_error(&format!("Error al invocar acción de notificación: {}", e));
        e
    })
}
