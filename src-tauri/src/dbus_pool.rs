use std::sync::Arc;
use tokio::sync::RwLock;
use zbus::Connection;

use crate::logger;

/// Shared D-Bus connection pool providing a single session and system bus
/// connection reused across all applets. Created before applet initialization
/// and registered as Tauri managed state.
pub struct DbusPool {
    session: Arc<RwLock<Option<Connection>>>,
    system: Arc<RwLock<Option<Connection>>>,
}

impl DbusPool {
    /// Initialize the pool by establishing both session and system bus connections.
    /// Each bus is attempted independently; if one fails, it is stored as `None`
    /// and the pool is still returned so callers never need to retry or panic.
    pub async fn init() -> Self {
        let session = match Connection::session().await {
            Ok(c) => {
                logger::log_info("DbusPool: conexión session establecida");
                Some(c)
            }
            Err(e) => {
                logger::log_info(&format!("DbusPool: no se pudo conectar a session bus: {e}"));
                None
            }
        };
        let system = match Connection::system().await {
            Ok(c) => {
                logger::log_info("DbusPool: conexión system establecida");
                Some(c)
            }
            Err(e) => {
                logger::log_info(&format!("DbusPool: no se pudo conectar a system bus: {e}"));
                None
            }
        };
        Self {
            session: Arc::new(RwLock::new(session)),
            system: Arc::new(RwLock::new(system)),
        }
    }

    /// Get a clone of the shared session bus connection.
    pub async fn session(&self) -> Option<Connection> {
        self.session.read().await.clone()
    }

    /// Get a clone of the shared system bus connection.
    pub async fn system(&self) -> Option<Connection> {
        self.system.read().await.clone()
    }

}
