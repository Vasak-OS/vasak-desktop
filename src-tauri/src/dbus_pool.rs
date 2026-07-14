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

    /// Attempt to reconnect the session bus. Replaces the stored connection
    /// on success or sets it to None on failure.
    pub async fn reconnect_session(&self) -> Result<(), zbus::Error> {
        let new_conn = Connection::session().await?;
        let mut guard = self.session.write().await;
        *guard = Some(new_conn);
        logger::log_info("DbusPool: reconexión session exitosa");
        Ok(())
    }

    /// Attempt to reconnect the system bus. Replaces the stored connection
    /// on success or sets it to None on failure.
    pub async fn reconnect_system(&self) -> Result<(), zbus::Error> {
        let new_conn = Connection::system().await?;
        let mut guard = self.system.write().await;
        *guard = Some(new_conn);
        logger::log_info("DbusPool: reconexión system exitosa");
        Ok(())
    }
}
