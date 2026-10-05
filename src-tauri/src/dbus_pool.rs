use serde::Deserialize;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::RwLock;
use zbus::zvariant::Type;
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

/// Llama un método en un servicio del bus de **sesión** y devuelve la respuesta
/// cruda.
///
/// El ayudante que comparten los clientes de los servicios propios de VasakOS
/// —`connect.rs` (vasak-connect) y `screen_time/health.rs`
/// (vasak-health-service)—: todos piden la conexión al pool compartido y llaman
/// igual, y una copia del mismo bloque por cliente es una copia que se separa.
/// Devuelve `Err` con un mensaje legible cuando no hay bus, que es lo que cada
/// cliente decide tragarse o no.
pub async fn session_call_raw<A>(
    app: &AppHandle,
    service: &str,
    path: &str,
    method: &str,
    args: &A,
) -> Result<zbus::Message, String>
where
    A: serde::ser::Serialize + Type,
{
    let Some(pool) = app.try_state::<DbusPool>() else {
        return Err("no hay conexión con el bus de sesión".to_string());
    };
    let Some(connection) = pool.session().await else {
        return Err("no hay conexión con el bus de sesión".to_string());
    };
    connection
        .call_method(Some(service), path, Some(service), method, args)
        .await
        .map_err(|err| err.to_string())
}

/// Como [`session_call_raw`], pero deserializa la respuesta al tipo esperado.
pub async fn session_call<A, R>(
    app: &AppHandle,
    service: &str,
    path: &str,
    method: &str,
    args: &A,
) -> Result<R, String>
where
    A: serde::ser::Serialize + Type,
    R: for<'d> Deserialize<'d> + Type,
{
    let reply = session_call_raw(app, service, path, method, args).await?;
    reply.body().deserialize().map_err(|err| err.to_string())
}
