use crate::structs::{Notification, NotificationUrgency};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use zbus::{interface, Connection};
use zbus::zvariant::Value;

// Global stores
static APP_HANDLE: Lazy<Arc<RwLock<Option<AppHandle>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
static NOTIFICATIONS: Lazy<Arc<RwLock<Vec<Notification>>>> = Lazy::new(|| Arc::new(RwLock::new(Vec::new())));

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
        drop(notifications);
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
static DBUS_CONNECTION: Lazy<Arc<RwLock<Option<Connection>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));

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
        
        // Icon mapping logic
        let icon_final = if !app_icon.is_empty() {
             app_icon
        } else {
             // Basic fallback mapping
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

        {
            let mut store = NOTIFICATIONS.write().await;
            store.insert(0, notification);
            if store.len() > MAX_NOTIFICATIONS {
                store.truncate(MAX_NOTIFICATIONS);
            }
        }
        
        emit_notifications_updated().await;
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
    
    // Request the org.freedesktop.Notifications name with ReplaceExisting
    use zbus::fdo::{RequestNameFlags, RequestNameReply};
    
    let reply = connection.request_name_with_flags(
        "org.freedesktop.Notifications",
        RequestNameFlags::ReplaceExisting | RequestNameFlags::DoNotQueue,
    ).await?;
    
    match reply {
        RequestNameReply::PrimaryOwner => {
            println!("Acquired org.freedesktop.Notifications successfully.");
        },
        RequestNameReply::InQueue => {
             println!("Queued for org.freedesktop.Notifications (another service is holding it).");
        },
        RequestNameReply::Exists => {
             println!("Failed to acquire org.freedesktop.Notifications: Name exists and replacement failed.");
        },
        RequestNameReply::AlreadyOwner => {
             println!("Already owner of org.freedesktop.Notifications.");
        },
    }

    // Register the object
    connection.object_server().at("/org/freedesktop/Notifications", NotificationServer).await?;
    
    // Also request a custom name for testing if primary fails (or always)
    let _ = connection.request_name("org.vasakos.Notifications").await;

    // Store connection
    {
        let mut guard = DBUS_CONNECTION.write().await;
        *guard = Some(connection.clone());
    }
    
    println!("Notification Server started on org.freedesktop.Notifications (and org.vasakos.Notifications)");
    
    // Keep the connection execution loop alive
    std::future::pending::<()>().await;
    
    Ok(())
}

// Adapter for existing signature in lib.rs?
// lib.rs calls `setup_notification_monitoring` which calls `start_notification_monitor`.
// We should update `start_notification_monitor` to call our new server.

pub async fn start_notification_monitor() -> Result<(), String> {
    tokio::spawn(async {
        if let Err(e) = start_notification_server().await {
             eprintln!("Error starting Notification Server: {}", e);
        }
    });
    Ok(())
}