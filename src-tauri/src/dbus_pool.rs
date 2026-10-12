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

    /// La conexión de sesión, y si no la hay —el bus no estaba al arrancar—,
    /// la intenta abrir y la deja guardada. Es para quien espera en bucle a que
    /// el bus aparezca: con `session()` esperaría para siempre.
    pub async fn session_or_connect(&self) -> Option<Connection> {
        if let Some(connection) = self.session().await {
            return Some(connection);
        }
        let mut slot = self.session.write().await;
        if slot.is_none() {
            match Connection::session().await {
                Ok(connection) => {
                    logger::log_info("DbusPool: conexión session establecida al reintentar");
                    *slot = Some(connection);
                }
                Err(e) => logger::log_info(&format!("DbusPool: sin bus de sesión todavía: {e}")),
            }
        }
        slot.clone()
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

/// Si `name` tiene dueño en el bus de `conn` o se puede activar.
///
/// Lo comparten el modo juego (`gamemoded`, en el de sesión) y «Mantener
/// despierto» (logind, en el del sistema). Cualquier error cuenta como que no
/// está: el ajuste se ve no disponible, nunca roto.
pub async fn name_on_bus(conn: &Connection, name: &str) -> bool {
    let Ok(dbus) = zbus::fdo::DBusProxy::new(conn).await else {
        return false;
    };
    let Ok(bus_name) = zbus::names::BusName::try_from(name) else {
        return false;
    };
    if dbus.name_has_owner(bus_name).await.unwrap_or(false) {
        return true;
    }
    dbus.list_activatable_names()
        .await
        .map(|names| names.iter().any(|n| n.as_str() == name))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::UnixStream;
    use zbus::{connection, Guid};

    async fn p2p() -> (Connection, Connection) {
        let guid = Guid::generate();
        let (client, server) = UnixStream::pair().unwrap();
        let server = connection::Builder::unix_stream(server)
            .server(guid)
            .unwrap()
            .p2p()
            .build();
        let client = connection::Builder::unix_stream(client).p2p().build();
        futures_util::try_join!(client, server).unwrap()
    }

    fn pool(session: Option<Connection>) -> DbusPool {
        DbusPool {
            session: Arc::new(RwLock::new(session)),
            system: Arc::new(RwLock::new(None)),
        }
    }

    /// Con una conexión guardada, se usa ésa y no se abre otra.
    #[tokio::test]
    async fn con_conexion_guardada_se_usa_esa() {
        let (client, _server) = p2p().await;
        let expected = client.server_guid().to_string();
        let pool = pool(Some(client));
        let got = pool.session_or_connect().await.expect("había una conexión");
        assert_eq!(got.server_guid().to_string(), expected);
    }

    /// Sin conexión guardada y sin bus a dónde ir, contesta que no hay y no
    /// guarda nada: el próximo intento vuelve a probar. Con `session()` a
    /// secas, quien espera el bus esperaría para siempre.
    #[tokio::test]
    async fn sin_bus_vuelve_a_intentar_en_cada_pedido() {
        // Una dirección que no existe: la prueba no depende de que la máquina
        // tenga bus de sesión.
        std::env::set_var(
            "DBUS_SESSION_BUS_ADDRESS",
            "unix:path=/nonexistent/vasak-desktop-test-bus",
        );
        let pool = pool(None);
        assert!(pool.session_or_connect().await.is_none());
        assert!(pool.session().await.is_none(), "guardó algo sin conectar");
        assert!(pool.session_or_connect().await.is_none());
    }
}
