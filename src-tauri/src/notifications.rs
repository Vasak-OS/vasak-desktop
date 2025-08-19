use crate::structs::{Notification, NotificationData, NotificationUrgency};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

static APP_HANDLE: Lazy<Arc<RwLock<Option<AppHandle>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));

static NOTIFICATIONS: Lazy<Arc<RwLock<Vec<Notification>>>> =
    Lazy::new(|| Arc::new(RwLock::new(Vec::new())));

const MAX_NOTIFICATIONS: usize = 50;

pub async fn initialize_app_handle(app_handle: AppHandle) {
    let mut handle = APP_HANDLE.write().await;
    *handle = Some(app_handle);
}

async fn emit_notifications_updated() {
    if let Some(app_handle) = APP_HANDLE.read().await.as_ref() {
        let notifications = NOTIFICATIONS.read().await;
        if let Err(e) = app_handle.emit("notifications-updated", &*notifications) {
            eprintln!("Error emitting notifications-updated event: {}", e);
        }
    }
}

pub async fn get_notifications() -> Result<Vec<Notification>, String> {
    let notifications = NOTIFICATIONS.read().await;
    Ok(notifications.clone())
}

pub async fn remove_notification(id: u32) -> Result<bool, String> {
    let mut notifications = NOTIFICATIONS.write().await;
    let initial_len = notifications.len();

    notifications.retain(|n| n.id != id);

    if notifications.len() < initial_len {
        drop(notifications); // Liberar el lock antes de emitir
        emit_notifications_updated().await;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn clear_all_notifications() -> Result<u32, String> {
    let mut notifications = NOTIFICATIONS.write().await;
    let count = notifications.len() as u32;
    notifications.clear();
    drop(notifications);

    emit_notifications_updated().await;
    Ok(count)
}

/// Función para agregar una notificación real al store
pub async fn add_real_notification(
    app_name: String,
    summary: String,
    body: String,
    app_icon: Option<String>,
    urgency: Option<NotificationUrgency>,
) -> Result<u32, String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let notification = Notification {
        id: timestamp as u32,
        app_name: app_name.clone(),
        summary,
        body,
        app_icon: app_icon.unwrap_or_else(|| match app_name.to_lowercase().as_str() {
            name if name.contains("chrome") => "google-chrome".to_string(),
            name if name.contains("kdeconnect") => "phone".to_string(),
            name => name.to_string(),
        }),
        timestamp,
        seen: false,
        urgency: urgency.unwrap_or(NotificationUrgency::Normal),
        actions: vec![],
        hints: HashMap::new(),
    };

    let notification_id = notification.id;

    {
        let mut notifications = NOTIFICATIONS.write().await;
        notifications.insert(0, notification);

        if notifications.len() > MAX_NOTIFICATIONS {
            notifications.truncate(MAX_NOTIFICATIONS);
        }
    }
    emit_notifications_updated().await;

    Ok(notification_id)
}

pub async fn start_notification_monitor() -> Result<(), String> {
    tokio::spawn(async {
        if let Err(e) = monitor_dbus_notifications().await {
            eprintln!("Error monitoring D-Bus notifications: {}", e);
        }
    });

    Ok(())
}

async fn monitor_dbus_notifications() -> Result<(), String> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    let mut cmd = Command::new("dbus-monitor")
        .arg("--session")
        .arg("interface=org.freedesktop.Notifications,member=Notify")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start dbus-monitor: {}", e))?;

    if let Some(stdout) = cmd.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        let mut current_notification: Option<NotificationData> = None;

        while let Ok(Some(line)) = lines.next_line().await {
            if line.contains("method call") && line.contains("Notify") {
                current_notification = Some(NotificationData::default());
            } else if let Some(ref mut notif) = current_notification {
                if line.trim().starts_with("string") {
                    let value = extract_string_value(&line);
                    if notif.app_name.is_empty() {
                        notif.app_name = value;
                    } else if notif.summary.is_empty() {
                        notif.summary = value;
                    } else if notif.body.is_empty() {
                        notif.body = value;

                        if !notif.app_name.is_empty() && !notif.summary.is_empty() {
                            let _ = add_real_notification(
                                notif.app_name.clone(),
                                notif.summary.clone(),
                                notif.body.clone(),
                                None,
                                Some(NotificationUrgency::Normal),
                            )
                            .await;
                        }

                        current_notification = None;
                    }
                }
            }
        }
    }

    Ok(())
}

fn extract_string_value(line: &str) -> String {
    if let Some(start) = line.find('"') {
        if let Some(end) = line.rfind('"') {
            if start < end {
                return line[start + 1..end].to_string();
            }
        }
    }
    String::new()
}

pub async fn send_system_notification(
    summary: String,
    body: Option<String>,
    urgency: Option<String>,
) -> Result<String, String> {
    use tokio::process::Command;

    let urgency_level = match urgency.as_deref() {
        Some("low") => "low",
        Some("critical") => "critical",
        _ => "normal",
    };

    let mut cmd = Command::new("notify-send");
    cmd.arg("--urgency").arg(urgency_level);
    cmd.arg(&summary);

    if let Some(body_text) = &body {
        cmd.arg(body_text);
    }

    match cmd.output().await {
        Ok(output) => {
            if output.status.success() {
                let urgency_enum = match urgency_level {
                    "low" => NotificationUrgency::Low,
                    "critical" => NotificationUrgency::Critical,
                    _ => NotificationUrgency::Normal,
                };

                let _ = add_real_notification(
                    "notify-send".to_string(),
                    summary,
                    body.unwrap_or_default(),
                    Some("dialog-information".to_string()),
                    Some(urgency_enum),
                )
                .await;

                Ok("Notification sent successfully".to_string())
            } else {
                Err(format!(
                    "Failed to send notification: {}",
                    String::from_utf8_lossy(&output.stderr)
                ))
            }
        }
        Err(e) => Err(format!("Failed to execute notify-send: {}", e)),
    }
}
