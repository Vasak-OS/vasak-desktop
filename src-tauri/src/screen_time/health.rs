//! El lado del escritorio de `vasak-health-service`: el cliente D-Bus.
//!
//! La contabilidad del tiempo de pantalla dejó de vivir en el escritorio y se
//! mudó al servicio de salud (su repositorio es `vasak-health-service`). El
//! escritorio sigue siendo el único que ve el compositor y la inactividad, así
//! que **empuja** las muestras —qué ventana está enfocada, si la pantalla está
//! bloqueada, cuándo empieza y termina la inactividad— y le **consulta** el
//! informe cuando el tablero lo pide. El servicio hace las cuentas, las guarda,
//! le pone categoría a cada aplicación y arranca solo al primer llamado (es
//! activable por D-Bus).
//!
//! # Sobre los tipos espejo
//!
//! El contrato vive una sola vez en el crate `vasak-health-protocol`, y
//! depender de él sería el sentido de tenerlo. No se puede todavía: ese crate
//! deriva `Type` del `zvariant` de zbus 5 y esta aplicación está clavada en
//! zbus 4 por el menú de la bandeja, así que los dos derives son incompatibles.
//! Los structs de abajo lo replican a mano, igual que `connect.rs` con
//! `vasak-connect`: si el contrato del servicio cambia, este archivo tiene que
//! cambiar con él, y nada lo va a recordar. Vale la pena revisarlo cuando
//! vasak-desktop suba a zbus 5.

use serde::Deserialize;
use tauri::{AppHandle, Manager};
use zbus::zvariant::Type;
use zbus::Connection;

use crate::dbus_pool::DbusPool;

/// Las direcciones del servicio en el bus de **sesión**: el tiempo de pantalla
/// es de quien está sentado en la máquina, y nada de esto sale de ahí.
const SERVICE: &str = "ar.net.vasak.os.Health";
const PATH: &str = "/ar/net/vasak/os/Health";

/// Cuántas cubetas tiene el desglose por hora: una por cada hora del día local.
pub const HOURS_IN_DAY: usize = 24;

// ── Tipos espejo del contrato (firma `(bssa(sa(sstat)))`) ──────────────────────

/// Cuánto usó una aplicación en un día. Espejo de `vasak_health_protocol::AppUsage`.
#[derive(Debug, Clone, Deserialize, Type)]
pub struct AppUsage {
    /// El `app-id` de la ventana, como lo da el compositor.
    pub app_id: String,
    /// La categoría freedesktop principal (`Network`, `Development`, `Game`…),
    /// o vacío si no se pudo averiguar.
    pub category: String,
    /// Milisegundos de uso ese día.
    pub millis: u64,
    /// Milisegundos por hora del día local: siempre [`HOURS_IN_DAY`] valores.
    /// Para datos traídos de la versión vieja pueden venir todos en cero aunque
    /// `millis` no lo esté.
    pub hours: Vec<u64>,
}

/// Un día con el uso de cada aplicación. Espejo de `vasak_health_protocol::DayUsage`.
#[derive(Debug, Clone, Deserialize, Type)]
pub struct DayUsage {
    /// La fecha local, `AAAA-MM-DD`.
    pub date: String,
    /// Las aplicaciones con algo de uso ese día.
    pub apps: Vec<AppUsage>,
}

/// Lo que devuelve una consulta de tiempo de pantalla. Espejo de
/// `vasak_health_protocol::ScreenTimeReport`.
#[derive(Debug, Clone, Deserialize, Type)]
pub struct ScreenTimeReport {
    /// Si el registro está prendido. Apagado, lo guardado se sigue pudiendo
    /// consultar, pero no se cuenta nada nuevo.
    pub enabled: bool,
    /// Hoy, en la zona horaria de la sesión: quien consulta no adivina el día.
    pub today: String,
    /// El primer día con algo guardado, o vacío si no hay historia.
    pub first_day: String,
    /// Sólo los días con algo, en orden. Los días vacíos no viajan.
    pub days: Vec<DayUsage>,
}

// ── El transporte, prestado del pool compartido ───────────────────────────────

/// Toma prestada la conexión de sesión compartida.
///
/// Devuelve `None` en vez de un error cuando el bus no está: para el escritorio,
/// «no hay servicio» y «no hay datos» son lo mismo —un tablero vacío— y una
/// muestra que no se pudo empujar no vale un diálogo.
async fn session(app: &AppHandle) -> Option<Connection> {
    app.try_state::<DbusPool>()?.session().await
}

/// Empuja algo al servicio sin mirar la respuesta (los métodos de ingreso no
/// devuelven nada). El servicio es activable por D-Bus: este llamado lo arranca
/// si todavía no corría.
async fn send<A>(app: &AppHandle, method: &str, args: &A) -> Result<(), String>
where
    A: serde::ser::Serialize + Type,
{
    let connection = session(app)
        .await
        .ok_or_else(|| "no hay conexión con el bus de sesión".to_string())?;
    connection
        .call_method(Some(SERVICE), PATH, Some(SERVICE), method, args)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

/// Llama un método que devuelve algo y deserializa la respuesta.
async fn call<A, R>(app: &AppHandle, method: &str, args: &A) -> Result<R, String>
where
    A: serde::ser::Serialize + Type,
    R: for<'d> Deserialize<'d> + Type,
{
    let connection = session(app)
        .await
        .ok_or_else(|| "no hay conexión con el bus de sesión".to_string())?;
    let reply = connection
        .call_method(Some(SERVICE), PATH, Some(SERVICE), method, args)
        .await
        .map_err(|err| err.to_string())?;
    reply.body().deserialize().map_err(|err| err.to_string())
}

// ── Ingreso: lo empuja el escritorio ──────────────────────────────────────────

/// Una muestra: a esta vuelta, ésta es la aplicación enfocada (vacío si
/// ninguna) y así está el bloqueo.
pub async fn push_sample(app: &AppHandle, app_id: &str, locked: bool) -> Result<(), String> {
    send(app, "PushSample", &(app_id, locked)).await
}

/// Empezó la inactividad hace `idle_ms` milisegundos.
pub async fn idle_started(app: &AppHandle, idle_ms: u64) -> Result<(), String> {
    send(app, "IdleStarted", &(idle_ms,)).await
}

/// Volvió la actividad.
pub async fn idle_ended(app: &AppHandle) -> Result<(), String> {
    send(app, "IdleEnded", &()).await
}

/// Se perdió la detección de inactividad.
pub async fn idle_monitor_lost(app: &AppHandle) -> Result<(), String> {
    send(app, "IdleMonitorLost", &()).await
}

/// Guarda todo lo contado sin esperar la ventana de descuento: antes de cerrar
/// la sesión o apagar.
pub async fn flush(app: &AppHandle) -> Result<(), String> {
    send(app, "Flush", &()).await
}

/// Prende o apaga el registro en el acto. El escritorio además persiste
/// `screen_time.enabled` en `vasak.conf`, que es de donde el servicio lo lee al
/// arrancar.
pub async fn set_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    send(app, "SetEnabled", &(enabled,)).await
}

// ── Consulta y control ────────────────────────────────────────────────────────

/// Los días de `from` a `to` (`AAAA-MM-DD`, los dos incluidos).
pub async fn screen_time(
    app: &AppHandle,
    from: &str,
    to: &str,
) -> Result<ScreenTimeReport, String> {
    call(app, "ScreenTime", &(from, to)).await
}

/// Borra todo el historial.
pub async fn clear_screen_time(app: &AppHandle) -> Result<(), String> {
    send(app, "ClearScreenTime", &()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Las firmas espejo tienen que ser **idénticas** a las del contrato, o el
    /// servicio manda bytes que el escritorio no sabe leer. La del informe es la
    /// que viaja entera: `(bssa(sa(sstat)))`.
    #[test]
    fn las_firmas_coinciden_con_el_contrato() {
        assert_eq!(AppUsage::signature().to_string(), "(sstat)");
        assert_eq!(DayUsage::signature().to_string(), "(sa(sstat))");
        assert_eq!(
            ScreenTimeReport::signature().to_string(),
            "(bssa(sa(sstat)))"
        );
    }
}
