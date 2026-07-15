use crate::structs::{Notification, NotificationUrgency};
use crate::logger::{log_debug, log_error, log_info, log_warning};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tauri::{AppHandle, Emitter};
use tokio::sync::{RwLock, Notify};
use zbus::{interface, Connection};
use zbus::zvariant::Value;

// Delta event types for efficient notification emission
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum NotificationDelta {
    Added { notification: Notification, dropped_id: Option<u32> },
    Removed { id: u32 },
    BatchUpdate { added: Vec<Notification>, removed: Vec<u32> },
    Cleared,
}

// Global stores
static APP_HANDLE: LazyLock<Arc<RwLock<Option<AppHandle>>>> = LazyLock::new(|| Arc::new(RwLock::new(None)));
static NOTIFICATIONS: LazyLock<Arc<RwLock<Vec<Notification>>>> = LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));
// Pending deltas accumulated during debounce window
static PENDING_DELTAS: LazyLock<Arc<RwLock<Vec<NotificationDelta>>>> = LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));
// Debounce notifier
static NOTIFY_UPDATE: LazyLock<Arc<Notify>> = LazyLock::new(|| Arc::new(Notify::new()));

const MAX_NOTIFICATIONS: usize = 50;

pub async fn initialize_app_handle(app_handle: AppHandle) {
    log_info("Inicializando sistema de notificaciones");
    let mut handle = APP_HANDLE.write().await;
    *handle = Some(app_handle);

    // Spawn the debouncer loop with delta coalescing
    tokio::spawn(async {
        let notify = NOTIFY_UPDATE.clone();
        loop {
            // Wait for a notification trigger
            notify.notified().await;

            // Trailing-edge debounce: wait 100ms of silence before emitting
            let mut deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(100);
            
            loop {
                tokio::select! {
                    _ = tokio::time::sleep_until(deadline) => {
                        // Timeout passed without new activity
                        break;
                    }
                    _ = notify.notified() => {
                        // New activity received, extend deadline
                        deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(100);
                    }
                }
            }

            // Emit coalesced deltas
            perform_emit_deltas().await;
        }
    });

}

// Queue a delta and trigger the debounced emission
async fn queue_delta(delta: NotificationDelta) {
    let mut pending = PENDING_DELTAS.write().await;
    pending.push(delta);
    drop(pending);
    NOTIFY_UPDATE.notify_one();
}

// The actual emission logic: coalesce pending deltas and emit
async fn perform_emit_deltas() {
    let mut pending = PENDING_DELTAS.write().await;
    if pending.is_empty() {
        return;
    }

    let deltas: Vec<NotificationDelta> = pending.drain(..).collect();
    drop(pending);

    if let Some(app_handle) = APP_HANDLE.read().await.as_ref() {
        let coalesced = if deltas.len() == 1 {
            deltas
        } else {
            coalesce_deltas(deltas)
        };

        for event_payload in &coalesced {
            log_debug(&format!("Emitiendo delta de notificaciones: {:?}", event_payload));
            if let Err(e) = app_handle.emit("notification-delta", event_payload) {
                log_error(&format!("Error al emitir evento notification-delta: {}", e));
            }
        }
    }
}

// Coalesce multiple deltas into ordered non-lossy deltas.
// - A Cleared boundary remains effective: if Cleared is followed by new additions,
//   a separate Cleared delta is emitted first, then the post-clear additions as a BatchUpdate.
// - Added items within each resulting delta are newest-first (reversed from arrival order).
fn coalesce_deltas(deltas: Vec<NotificationDelta>) -> Vec<NotificationDelta> {
    let mut added: Vec<Notification> = Vec::new();
    let mut removed: Vec<u32> = Vec::new();
    let mut cleared = false;

    for delta in deltas {
        match delta {
            NotificationDelta::Cleared => {
                added.clear();
                removed.clear();
                cleared = true;
            }
            NotificationDelta::Added { notification, dropped_id } => {
                added.push(notification);
                if let Some(id) = dropped_id {
                    removed.push(id);
                }
            }
            NotificationDelta::Removed { id } => {
                removed.push(id);
            }
            NotificationDelta::BatchUpdate { added: batch_added, removed: batch_removed } => {
                added.extend(batch_added);
                removed.extend(batch_removed);
            }
        }
    }

    let mut result = Vec::new();

    // If we saw a Cleared, emit it first so the frontend resets before additions.
    if cleared {
        result.push(NotificationDelta::Cleared);
    }

    // Emit accumulated adds/removes as a BatchUpdate (newest-first).
    if !added.is_empty() || !removed.is_empty() {
        added.reverse();
        result.push(NotificationDelta::BatchUpdate { added, removed });
    }

    result
}

pub async fn get_notifications() -> Result<Vec<Notification>, String> {
    let notifications = NOTIFICATIONS.read().await;
    // Cap at MAX_NOTIFICATIONS (50) without cloning the entire Vec
    let count = notifications.len().min(MAX_NOTIFICATIONS);
    Ok(notifications[..count].to_vec())
}

pub async fn remove_notification(id: u32) -> Result<bool, String> {
    log_info(&format!("Eliminando notificación con ID: {}", id));
    let mut notifications = NOTIFICATIONS.write().await;
    let initial_len = notifications.len();
    notifications.retain(|n| n.id != id);

    if notifications.len() < initial_len {
        log_debug(&format!("Notificación {} eliminada correctamente", id));
        drop(notifications);
        queue_delta(NotificationDelta::Removed { id }).await;
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
    queue_delta(NotificationDelta::Cleared).await;
    Ok(count)
}

pub async fn send_system_notification(
    summary: String,
    body: Option<String>,
    urgency: Option<String>,
) -> Result<String, String> {
       // Re-use internal logic or simply add to store, since we ARE the server now.
       // Calling internal add method directly.
       let urgency_enum = match urgency.as_deref() {
            Some("low") => NotificationUrgency::Low,
            Some("critical") => NotificationUrgency::Critical,
            _ => NotificationUrgency::Normal,
        };

        let _ = NotificationServer::add_notification_internal(
            "VasakOS".to_string(),
            summary,
            body.unwrap_or_default(),
            String::new(), // icon
            urgency_enum,
            vec![], // actions
            HashMap::new()
        ).await;

        Ok("Notification added".to_string())
}

// --------------------------------------------------------------------------------
// ZBus Notification Server Implementation
// --------------------------------------------------------------------------------

// Global connection storage
static DBUS_CONNECTION: LazyLock<Arc<RwLock<Option<Connection>>>> = LazyLock::new(|| Arc::new(RwLock::new(None)));

#[derive(Clone)]
struct NotificationServer;

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationServer {
    async fn get_capabilities(&self) -> Vec<String> {
        vec![
            "body".to_string(),
            "actions".to_string(),
            "persistence".to_string(),
            "icon-static".to_string(),
        ]
    }
    
    // Define signals
    #[zbus(signal)]
    async fn action_invoked(ctxt: &zbus::object_server::SignalContext<'_>, id: u32, action_key: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn notification_closed(ctxt: &zbus::object_server::SignalContext<'_>, id: u32, reason: u32) -> zbus::Result<()>;

    async fn get_server_information(&self) -> (String, String, String, String) {
        (
            "VasakOS Notification Server".to_string(),
            "VasakOS".to_string(),
            "0.1.0".to_string(),
            "1.2".to_string(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &self,
        app_name: String,
        _replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, Value<'_>>,
        _expire_timeout: i32,
    ) -> u32 {
        let urgency = if let Some(Value::U8(u)) = hints.get("urgency") {
             match u {
                 0 => NotificationUrgency::Low,
                 1 => NotificationUrgency::Normal,
                 2 => NotificationUrgency::Critical,
                 _ => NotificationUrgency::Normal,
             }
        } else {
             NotificationUrgency::Normal
        };

        NotificationServer::add_notification_internal(
            app_name,
            summary,
            body,
            app_icon,
            urgency,
            actions,
            HashMap::new()
        ).await
    }

    async fn close_notification(&self, id: u32) {
        let _ = remove_notification(id).await;
    }
}

impl NotificationServer {
    async fn add_notification_internal(
        app_name: String,
        summary: String,
        body: String,
        app_icon: String,
        urgency: NotificationUrgency,
        actions: Vec<String>,
        _hints: HashMap<String, String>
    ) -> u32 {
         let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let icon_final = if !app_icon.is_empty() {
             app_icon
        } else {
             let name_lower = app_name.to_lowercase();
             if name_lower.contains("chrome") { "google-chrome".to_string() }
             else if name_lower.contains("telegram") { "telegram-desktop".to_string() }
             else { name_lower }
        };

        let notification = Notification {
            id: timestamp as u32, 
            app_name,
            summary,
            body,
            app_icon: icon_final,
            timestamp,
            seen: false,
            urgency,
            actions,
            hints: HashMap::new(),
        };

        let id = notification.id;
        let notification_clone = notification.clone();

        let dropped_id = {
            let mut store = NOTIFICATIONS.write().await;
            store.insert(0, notification);
            if store.len() > MAX_NOTIFICATIONS {
                // Capture the ID of the evicted (oldest) entry before truncating
                let dropped = store.last().map(|n| n.id);
                store.truncate(MAX_NOTIFICATIONS);
                dropped
            } else {
                None
            }
        };
        
        queue_delta(NotificationDelta::Added { notification: notification_clone, dropped_id }).await;
        id
    }
}

pub async fn invoke_action(id: u32, action_key: String) -> Result<(), String> {
    let conn_guard = DBUS_CONNECTION.read().await;
    if let Some(conn) = conn_guard.as_ref() {
        let iface_ref = conn.object_server().interface::<_, NotificationServer>("/org/freedesktop/Notifications").await
             .map_err(|e| format!("Failed to get interface: {}", e))?;
             
        let ctxt = iface_ref.signal_context(); 
        NotificationServer::action_invoked(ctxt, id, &action_key).await
             .map_err(|e| format!("Failed to emit signal: {}", e))?;
             
        Ok(())
    } else {
        Err("DBus connection not initialized".to_string())
    }
}

pub async fn start_notification_server() -> Result<(), Box<dyn std::error::Error>> {
    let connection = Connection::session().await?;
    
    use zbus::fdo::{RequestNameFlags, RequestNameReply};
    
    let reply = connection.request_name_with_flags(
        "org.freedesktop.Notifications",
        RequestNameFlags::ReplaceExisting | RequestNameFlags::DoNotQueue,
    ).await?;
    
    match reply {
        RequestNameReply::PrimaryOwner => {
            log_info("Acquired org.freedesktop.Notifications successfully.");
        },
        RequestNameReply::InQueue => {
             log_warning("Queued for org.freedesktop.Notifications (another service is holding it).");
        },
        RequestNameReply::Exists => {
             log_error("Failed to acquire org.freedesktop.Notifications: Name exists and replacement failed.");
        },
        RequestNameReply::AlreadyOwner => {
             log_info("Already owner of org.freedesktop.Notifications.");
        },
    }

    connection.object_server().at("/org/freedesktop/Notifications", NotificationServer).await?;
    let _ = connection.request_name("org.vasakos.Notifications").await;

    {
        let mut guard = DBUS_CONNECTION.write().await;
        *guard = Some(connection.clone());
    }
    
    log_info("Notification Server started");
    
    std::future::pending::<()>().await;
    
    Ok(())
}

