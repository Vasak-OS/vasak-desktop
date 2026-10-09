//! Bloquear la pantalla a pedido (vasak-desktop#190, ancla #174).
//!
//! El escritorio no bloquea nada por su cuenta: le pide a logind que bloquee la
//! sesión (`org.freedesktop.login1.Session.Lock`, lo mismo que
//! `loginctl lock-session`) y logind emite la señal `Lock`. La escucha
//! `vasak-lock-listener.service` (vasak-desktop-settings), que lanza el bloqueo
//! de siempre —`vasak-lock-screen`, por `ext-session-lock`—, el mismo que el de
//! inactividad y el de Super+L. Así hay un solo camino para bloquear, y
//! cualquier otro programa que lo pida como lo pide el resto de Linux recibe el
//! mismo bloqueo.
//!
//! Sin subprocesos: una llamada por el bus del sistema, con la conexión
//! compartida de `DbusPool`.
//!
//! # Qué sesión
//!
//! `/org/freedesktop/login1/session/auto`. El escritorio corre como servicio
//! del usuario (uwsm), fuera del ámbito de la sesión, así que `self` no
//! resuelve; `auto` es la sesión del proceso si tiene una y, si no, la sesión
//! gráfica del usuario. Es la misma que resuelve el swayidle que escucha,
//! que pide `GetSession("auto")` por la misma razón.
//!
//! Bloquear la sesión propia no pide permiso a polkit: logind lo deja hacer al
//! dueño de la sesión.

use tauri::{AppHandle, Manager};
use zbus::Connection;

use crate::dbus_pool::DbusPool;
use crate::logger::{log_error, log_info};

const LOGIND: &str = "org.freedesktop.login1";
/// La sesión gráfica del usuario, la del escritorio.
const DISPLAY_SESSION: &str = "/org/freedesktop/login1/session/auto";
const SESSION_INTERFACE: &str = "org.freedesktop.login1.Session";

/// Pide a logind que bloquee la sesión gráfica del usuario.
pub(crate) async fn request_lock(connection: &Connection) -> Result<(), String> {
    connection
        .call_method(
            Some(LOGIND),
            DISPLAY_SESSION,
            Some(SESSION_INTERFACE),
            "Lock",
            &(),
        )
        .await
        .map(|_| ())
        .map_err(|error| format!("logind no bloqueó la sesión: {error}"))
}

/// Si hay a quién pedirle el bloqueo: logind en el bus y una sesión gráfica
/// del usuario. Lee el `Id` de la sesión; si logind no está, o el usuario no
/// tiene sesión (un escritorio lanzado a mano desde otro lado), no hay.
pub(crate) async fn lock_available(connection: &Connection) -> bool {
    connection
        .call_method(
            Some(LOGIND),
            DISPLAY_SESSION,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(SESSION_INTERFACE, "Id"),
        )
        .await
        .is_ok()
}

async fn system_bus(app: &AppHandle) -> Option<Connection> {
    app.try_state::<DbusPool>()?.system().await
}

/// Bloquea la pantalla. Sin confirmación: bloquear no pierde nada.
#[tauri::command]
pub async fn lock_screen(app: AppHandle) -> Result<(), String> {
    let Some(connection) = system_bus(&app).await else {
        let message = "no hay bus del sistema: no se puede pedir el bloqueo".to_string();
        log_error(&message);
        return Err(message);
    };
    log_info("Bloqueando la sesión (logind Session.Lock)");
    request_lock(&connection).await.inspect_err(|error| {
        log_error(error);
    })
}

/// Si el botón Bloquear puede hacer algo; sin logind se ve no disponible.
#[tauri::command]
pub async fn lock_screen_available(app: AppHandle) -> bool {
    match system_bus(&app).await {
        Some(connection) => lock_available(&connection).await,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tray::bus_tests::PrivateBus;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use zbus::interface;

    /// La sesión de un logind de mentira: cuenta los pedidos de bloqueo.
    struct FakeSession {
        locks: Arc<AtomicU32>,
        unlocks: Arc<AtomicU32>,
    }

    #[interface(name = "org.freedesktop.login1.Session")]
    impl FakeSession {
        fn lock(&self) {
            self.locks.fetch_add(1, Ordering::SeqCst);
        }

        fn unlock(&self) {
            self.unlocks.fetch_add(1, Ordering::SeqCst);
        }

        #[zbus(property)]
        fn id(&self) -> String {
            "33".to_string()
        }
    }

    /// A propósito la ruta escrita y no la constante: si el código pidiera a
    /// otra sesión (`self`, que no resuelve fuera del ámbito de la sesión), la
    /// prueba tiene que verlo.
    const AUTO: &str = "/org/freedesktop/login1/session/auto";

    struct Counters {
        locks: Arc<AtomicU32>,
        unlocks: Arc<AtomicU32>,
    }

    /// Sirve la sesión en `path` con el nombre de logind.
    async fn serve_logind(bus: &PrivateBus, path: &'static str) -> (Connection, Counters) {
        let locks = Arc::new(AtomicU32::new(0));
        let unlocks = Arc::new(AtomicU32::new(0));
        let session = FakeSession {
            locks: locks.clone(),
            unlocks: unlocks.clone(),
        };
        let connection = bus
            .connect_serving(|builder| builder.serve_at(path, session)?.name(LOGIND))
            .await;
        (connection, Counters { locks, unlocks })
    }

    #[tokio::test]
    async fn bloquear_pide_lock_a_la_sesion_grafica_de_logind() {
        let bus = crate::tray::bus_tests::private_bus!();
        let (_logind, counters) = serve_logind(&bus, AUTO).await;
        let client = bus.connect().await;

        request_lock(&client).await.expect("logind contesta");

        assert_eq!(
            counters.locks.load(Ordering::SeqCst),
            1,
            "un pedido, un Lock"
        );
        assert_eq!(counters.unlocks.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn con_logind_y_sesion_esta_disponible() {
        let bus = crate::tray::bus_tests::private_bus!();
        let (_logind, counters) = serve_logind(&bus, AUTO).await;
        let client = bus.connect().await;

        assert!(lock_available(&client).await);
        // Mirar si se puede no es pedirlo.
        assert_eq!(counters.locks.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn sin_logind_no_esta_disponible_y_pedirlo_falla() {
        let bus = crate::tray::bus_tests::private_bus!();
        let client = bus.connect().await;

        assert!(!lock_available(&client).await);
        let error = request_lock(&client).await.expect_err("nadie contesta");
        assert!(error.contains("logind no bloqueó"), "{error}");
    }

    /// logind está pero el usuario no tiene sesión gráfica (`auto` no
    /// resuelve): el botón no puede hacer nada, y lo dice.
    #[tokio::test]
    async fn sin_sesion_grafica_no_esta_disponible() {
        let bus = crate::tray::bus_tests::private_bus!();
        let (_logind, counters) = serve_logind(&bus, "/org/freedesktop/login1/session/_7").await;
        let client = bus.connect().await;

        assert!(!lock_available(&client).await);
        assert!(request_lock(&client).await.is_err());
        assert_eq!(counters.locks.load(Ordering::SeqCst), 0);
    }
}
