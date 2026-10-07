//! La luz nocturna, del lado del escritorio (vasak-desktop#178).
//!
//! `wlsunset` corre como la unidad de usuario `vasak-nightlight.service`, no
//! como hijo del escritorio: así no muere con él y vuelve sola al iniciar la
//! sesión si quedó habilitada. La configuración (temperaturas y horario) vive
//! en la línea `ExecStart` de esa unidad y la lee y la escribe
//! `tauri-plugin-display-manager`, el mismo que usa vasak-settings. Este módulo
//! sólo la prende, la apaga y la sigue.
//!
//! - **Por el D-Bus de systemd, sin subprocesos:** `StartUnit`/`StopUnit` para
//!   ahora, `EnableUnitFiles`/`DisableUnitFiles` para el próximo inicio de
//!   sesión, y `Reload` sólo cuando systemd dice que su copia de la unidad
//!   quedó vieja (`NeedDaemonReload`).
//! - **Sin sondeos:** el estado se sigue por `PropertiesChanged` de la unidad
//!   (`ActiveState`). systemd manda esas señales sólo a quien se suscribió
//!   (`Subscribe`), y el seguimiento corre **sólo con el centro de control
//!   abierto**: al cerrarlo se corta y se desuscribe. Con el centro cerrado no
//!   queda nada escuchando.
//! - **`wlsunset` corre sólo si la luz está encendida:** apagarla detiene la
//!   unidad, no la deja ociosa.

use std::sync::OnceLock;

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_display_manager::night_light as config;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zbus::{Connection, Proxy};

use crate::dbus_pool::DbusPool;
use crate::logger::{log_error, log_info};

const SYSTEMD_DEST: &str = "org.freedesktop.systemd1";
const MANAGER_PATH: &str = "/org/freedesktop/systemd1";
const MANAGER_IFACE: &str = "org.freedesktop.systemd1.Manager";
const UNIT_IFACE: &str = "org.freedesktop.systemd1.Unit";
/// Lo que contesta systemd por una unidad cuyo archivo no existe: la luz
/// nocturna nunca se guardó.
const NO_SUCH_UNIT: &str = "org.freedesktop.systemd1.NoSuchUnit";
/// Lo que devuelven `EnableUnitFiles` y `DisableUnitFiles`: qué enlaces tocó
/// (tipo, enlace, destino).
type UnitFileChanges = Vec<(String, String, String)>;
/// Lo que contesta `Subscribe` si esta conexión ya estaba suscrita.
const ALREADY_SUBSCRIBED: &str = "org.freedesktop.systemd1.AlreadySubscribed";

/// La unidad que corre `wlsunset`. Es la del plugin: la misma que escribe la
/// configuración y que prende vasak-settings.
pub const UNIT: &str = config::UNIT_NAME;

/// El evento que reciben las ventanas con cada cambio, con un
/// [`NightLightState`] adentro.
pub const CHANGED_EVENT: &str = "night-light-changed";

/// Lo que se sabe de la luz nocturna.
///
/// `available` es falso sin `wlsunset` instalado: el mosaico se ve **no
/// disponible**, nunca roto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NightLightState {
    pub available: bool,
    pub enabled: bool,
}

/// La ruta del objeto de una unidad en el bus de systemd.
///
/// Es la regla de `bus_label_escape` de systemd: queda todo lo que es letra, y
/// los dígitos salvo el primero; el resto va como `_` y dos cifras
/// hexadecimales en minúscula. Calcularla ahorra un `LoadUnit` antes de cada
/// lectura, y sirve también para suscribirse a una unidad que todavía no está
/// cargada.
pub fn unit_object_path(unit: &str) -> String {
    let mut path = String::from("/org/freedesktop/systemd1/unit/");
    if unit.is_empty() {
        path.push('_');
        return path;
    }
    for (index, byte) in unit.bytes().enumerate() {
        if byte.is_ascii_alphabetic() || (index > 0 && byte.is_ascii_digit()) {
            path.push(byte as char);
        } else {
            path.push_str(&format!("_{byte:02x}"));
        }
    }
    path
}

/// Si un `ActiveState` cuenta como encendida. Mientras arranca ya lo es: el
/// mosaico no tiene que parpadear entre el toque y el `active`.
pub fn is_on(active_state: &str) -> bool {
    matches!(active_state, "active" | "activating" | "reloading")
}

fn is_error(error: &zbus::Error, name: &str) -> bool {
    matches!(error, zbus::Error::MethodError(found, _, _) if found.as_str() == name)
}

async fn manager(conn: &Connection) -> zbus::Result<Proxy<'static>> {
    zbus::proxy::Builder::new(conn)
        .destination(SYSTEMD_DEST)?
        .path(MANAGER_PATH)?
        .interface(MANAGER_IFACE)?
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
}

async fn unit_proxy(conn: &Connection, unit: &str) -> zbus::Result<Proxy<'static>> {
    zbus::proxy::Builder::new(conn)
        .destination(SYSTEMD_DEST)?
        .path(unit_object_path(unit))?
        .interface(UNIT_IFACE)?
        // Sin caché: una caché pediría todas las propiedades de la unidad —son
        // decenas— para leer una.
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
}

/// Si la unidad está encendida, preguntándole a systemd.
pub async fn read_enabled(conn: &Connection, unit: &str) -> zbus::Result<bool> {
    let state: String = unit_proxy(conn, unit)
        .await?
        .get_property("ActiveState")
        .await?;
    Ok(is_on(&state))
}

/// Relee las unidades sólo si hace falta: si la de la luz no está cargada
/// (recién creada) o si su archivo cambió desde que systemd lo leyó. `Reload`
/// relee **todas** las unidades del usuario; no se hace por las dudas.
async fn reload_if_needed(conn: &Connection, unit: &str) -> zbus::Result<()> {
    let proxy = unit_proxy(conn, unit).await?;
    let loaded = proxy.get_property::<String>("LoadState").await? == "loaded";
    let stale = proxy
        .get_property::<bool>("NeedDaemonReload")
        .await
        .unwrap_or(true);
    if !loaded || stale {
        manager(conn).await?.call::<_, _, ()>("Reload", &()).await?;
    }
    Ok(())
}

/// Prende la luz: la deja habilitada para el próximo inicio de sesión y la
/// arranca ahora. La unidad ya tiene que existir (ver [`ensure_unit`]).
pub async fn switch_on(conn: &Connection, unit: &str) -> zbus::Result<()> {
    reload_if_needed(conn, unit).await?;
    let manager = manager(conn).await?;
    // `runtime = false`: en `~/.config`, que sobrevive al reinicio.
    // `force = false`: no pisa un enlace que el usuario haya hecho a otra cosa.
    manager
        .call::<_, _, (bool, UnitFileChanges)>("EnableUnitFiles", &(vec![unit], false, false))
        .await?;
    manager
        .call::<_, _, zbus::zvariant::OwnedObjectPath>("StartUnit", &(unit, "replace"))
        .await?;
    Ok(())
}

/// Apaga la luz: deshabilitada primero —si eso falla, volvería en el próximo
/// inicio de sesión aunque ahora se detenga— y detenida después. Una unidad
/// que no existe es una luz que nunca se encendió: no es un error.
pub async fn switch_off(conn: &Connection, unit: &str) -> zbus::Result<()> {
    let manager = manager(conn).await?;
    match manager
        .call::<_, _, UnitFileChanges>("DisableUnitFiles", &(vec![unit], false))
        .await
    {
        Err(e) if !is_error(&e, NO_SUCH_UNIT) => return Err(e),
        _ => {}
    }
    match manager
        .call::<_, _, zbus::zvariant::OwnedObjectPath>("StopUnit", &(unit, "replace"))
        .await
    {
        Err(e) if !is_error(&e, NO_SUCH_UNIT) => Err(e),
        _ => Ok(()),
    }
}

/// Aplica una configuración recién guardada. `wlsunset` lee sus argumentos al
/// arrancar, así que si está corriendo se reinicia; si está apagada no hace
/// nada: el próximo encendido relee la unidad.
pub async fn apply_config(conn: &Connection, unit: &str) -> zbus::Result<bool> {
    if !read_enabled(conn, unit).await? {
        return Ok(false);
    }
    reload_if_needed(conn, unit).await?;
    manager(conn)
        .await?
        .call::<_, _, zbus::zvariant::OwnedObjectPath>("RestartUnit", &(unit, "replace"))
        .await?;
    Ok(true)
}

/// Sigue la unidad hasta que la conexión se corta: se suscribe a las señales
/// de systemd, lee el estado y lo vuelve a contar cuando cambia
/// `ActiveState`. `on_change` recibe si quedó encendida.
pub async fn follow(conn: &Connection, unit: &str, on_change: impl Fn(bool)) -> zbus::Result<()> {
    match manager(conn)
        .await?
        .call::<_, _, ()>("Subscribe", &())
        .await
    {
        Err(e) if !is_error(&e, ALREADY_SUBSCRIBED) => return Err(e),
        _ => {}
    }
    let properties = PropertiesProxy::builder(conn)
        .destination(SYSTEMD_DEST)?
        .path(unit_object_path(unit))?
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await?;
    let mut changes = properties
        .receive_properties_changed_with_args(&[(0, UNIT_IFACE)])
        .await?;

    // Lo primero, después de suscribirse: así un cambio entre la lectura y la
    // suscripción no se pierde.
    on_change(read_enabled(conn, unit).await?);

    let iface = InterfaceName::from_static_str_unchecked(UNIT_IFACE);
    while let Some(change) = changes.next().await {
        let Ok(args) = change.args() else { continue };
        if args.interface_name() != &iface {
            continue;
        }
        let state = match args
            .changed_properties()
            .get("ActiveState")
            .map(|value| <&str>::try_from(value).ok().map(is_on))
        {
            // Cambió otra propiedad: no es asunto del mosaico.
            None => continue,
            Some(Some(on)) => on,
            // Invalidada o con otro tipo: se pregunta.
            Some(None) => read_enabled(conn, unit).await?,
        };
        on_change(state);
    }
    Ok(())
}

/// Le saca a systemd la suscripción de esta conexión. Que no estuviera
/// suscrita no es un error.
async fn unsubscribe(conn: &Connection) {
    if let Ok(manager) = manager(conn).await {
        let _ = manager.call::<_, _, ()>("Unsubscribe", &()).await;
    }
}

// ── El estado compartido del proceso ─────────────────────────────────────────

/// Lo último que se publicó, para no avisar dos veces lo mismo.
static PUBLISHED: OnceLock<std::sync::Mutex<Option<NightLightState>>> = OnceLock::new();

/// El seguimiento en curso, si el centro está abierto. Un `Mutex` asíncrono
/// para que abrir y cerrar el centro rápido no crucen sus llamadas a systemd:
/// un `Unsubscribe` del cierre anterior no puede llegar después del
/// `Subscribe` de la apertura nueva.
static WATCH: Mutex<Option<JoinHandle<()>>> = Mutex::const_new(None);

fn available() -> bool {
    config::wlsunset_available()
}

/// Manda el estado a las ventanas, si cambió.
fn publish(app: &AppHandle, state: NightLightState) {
    let last = PUBLISHED.get_or_init(|| std::sync::Mutex::new(None));
    let Ok(mut last) = last.lock() else { return };
    if *last == Some(state) {
        return;
    }
    *last = Some(state);
    log_info(&format!(
        "Luz nocturna: {}",
        if state.enabled {
            "encendida"
        } else {
            "apagada"
        }
    ));
    let _ = app.emit(CHANGED_EVENT, state);
}

async fn session(app: &AppHandle) -> Result<Connection, String> {
    match app.try_state::<DbusPool>() {
        Some(pool) => pool
            .session()
            .await
            .ok_or_else(|| "no hay conexión con el bus de sesión".to_string()),
        None => Err("no hay conexión con el bus de sesión".to_string()),
    }
}

/// Crea la unidad con la configuración por omisión si todavía no existe: la
/// primera vez que se toca el mosaico, antes de haber pasado por el detalle o
/// por Configuración.
fn ensure_unit() -> Result<(), String> {
    let path = config::unit_path().map_err(|e| e.to_string())?;
    let saved = config::read(&path);
    if !saved.configured {
        config::write(&path, &saved.config).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Empieza a seguir la unidad. Lo llama el centro de control al abrirse.
pub fn watch(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut slot = WATCH.lock().await;
        if slot.as_ref().is_some_and(|task| !task.is_finished()) {
            return;
        }
        if !available() {
            publish(
                &app,
                NightLightState {
                    available: false,
                    enabled: false,
                },
            );
            return;
        }
        let conn = match session(&app).await {
            Ok(conn) => conn,
            Err(e) => {
                log_error(&format!("No se pudo seguir la luz nocturna: {e}"));
                return;
            }
        };
        let follower = app.clone();
        *slot = Some(tokio::spawn(async move {
            let result = follow(&conn, UNIT, |enabled| {
                publish(
                    &follower,
                    NightLightState {
                        available: true,
                        enabled,
                    },
                )
            })
            .await;
            if let Err(e) = result {
                log_error(&format!("Se cortó el seguimiento de la luz nocturna: {e}"));
            }
        }));
    });
}

/// Deja de seguir la unidad y le saca la suscripción a systemd. Lo llama el
/// centro de control al cerrarse.
pub fn unwatch(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut slot = WATCH.lock().await;
        let Some(task) = slot.take() else { return };
        task.abort();
        let _ = task.await;
        if let Ok(conn) = session(&app).await {
            unsubscribe(&conn).await;
        }
    });
}

// ── Comandos ─────────────────────────────────────────────────────────────────

/// El estado de la luz nocturna. Una lectura por el bus: se pide al montar el
/// mosaico, y después el estado llega por el evento.
#[tauri::command]
pub async fn get_night_light_state(app: AppHandle) -> Result<NightLightState, String> {
    if !available() {
        return Ok(NightLightState {
            available: false,
            enabled: false,
        });
    }
    let conn = session(&app).await?;
    let enabled = read_enabled(&conn, UNIT)
        .await
        .map_err(|e| format!("systemd no contestó por la luz nocturna: {e}"))?;
    Ok(NightLightState {
        available: true,
        enabled,
    })
}

/// Prende o apaga la luz nocturna con la configuración guardada. Devuelve el
/// estado que había, como los demás interruptores del centro.
#[tauri::command]
pub async fn set_night_light_enabled(app: AppHandle, enabled: bool) -> Result<bool, String> {
    if !available() {
        return Err("wlsunset no está instalado".to_string());
    }
    let conn = session(&app).await?;
    let previous = read_enabled(&conn, UNIT).await.unwrap_or(false);
    let result = if enabled {
        ensure_unit()?;
        switch_on(&conn, UNIT).await
    } else {
        switch_off(&conn, UNIT).await
    };
    result.map_err(|e| {
        let message = format!("systemd no aceptó el cambio de la luz nocturna: {e}");
        log_error(&message);
        message
    })?;
    publish(
        &app,
        NightLightState {
            available: true,
            enabled,
        },
    );
    Ok(previous)
}

/// Aplica la configuración que se acaba de guardar con el plugin: si la luz
/// está encendida, la reinicia para que `wlsunset` tome los valores nuevos.
/// Devuelve si hubo que reiniciarla.
#[tauri::command]
pub async fn apply_night_light(app: AppHandle) -> Result<bool, String> {
    if !available() {
        return Ok(false);
    }
    let conn = session(&app).await?;
    apply_config(&conn, UNIT)
        .await
        .map_err(|e| format!("no se pudo aplicar la luz nocturna: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tray::bus_tests::{private_bus, PrivateBus};
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::Duration;
    use zbus::interface;
    use zbus::object_server::ObjectServer;
    use zbus::zvariant::OwnedObjectPath;

    /// Lo que se le pidió al systemd de mentira, en orden.
    type Calls = Arc<StdMutex<Vec<String>>>;

    /// Una unidad de mentira, con las tres propiedades que se leen.
    struct FakeUnit {
        active_state: String,
        load_state: String,
        need_daemon_reload: bool,
    }

    #[interface(name = "org.freedesktop.systemd1.Unit")]
    impl FakeUnit {
        #[zbus(property)]
        fn active_state(&self) -> String {
            self.active_state.clone()
        }
        #[zbus(property)]
        fn load_state(&self) -> String {
            self.load_state.clone()
        }
        #[zbus(property)]
        fn need_daemon_reload(&self) -> bool {
            self.need_daemon_reload
        }
    }

    /// El administrador de systemd de mentira: anota cada llamada con la
    /// unidad que nombra, y arrancar o detener cambia el estado de la unidad y
    /// lo avisa por `PropertiesChanged`, como el de verdad.
    struct FakeManager {
        calls: Calls,
        /// Las unidades que existen. Las otras contestan `NoSuchUnit`.
        known: Vec<String>,
    }

    impl FakeManager {
        fn note(&self, call: String) {
            self.calls.lock().unwrap().push(call);
        }

        fn check(&self, unit: &str) -> zbus::fdo::Result<()> {
            if self.known.iter().any(|known| known == unit) {
                Ok(())
            } else {
                Err(zbus::fdo::Error::Failed("NoSuchUnit".into()))
            }
        }

        async fn set_state(server: &ObjectServer, unit: &str, state: &str) {
            let Ok(iface) = server
                .interface::<_, FakeUnit>(unit_object_path(unit))
                .await
            else {
                return;
            };
            iface.get_mut().await.active_state = state.to_string();
            iface
                .get()
                .await
                .active_state_changed(iface.signal_context())
                .await
                .unwrap();
        }
    }

    /// Un error con el nombre que usa systemd, que es lo que el módulo mira.
    #[derive(Debug, zbus::DBusError)]
    #[zbus(prefix = "org.freedesktop.systemd1")]
    enum SystemdError {
        #[zbus(error)]
        ZBus(zbus::Error),
        NoSuchUnit(String),
    }

    #[interface(name = "org.freedesktop.systemd1.Manager")]
    impl FakeManager {
        fn subscribe(&self) {
            self.note("Subscribe".into());
        }
        fn unsubscribe(&self) {
            self.note("Unsubscribe".into());
        }
        fn reload(&self) {
            self.note("Reload".into());
        }
        fn enable_unit_files(
            &self,
            files: Vec<String>,
            runtime: bool,
            force: bool,
        ) -> Result<(bool, UnitFileChanges), SystemdError> {
            self.note(format!("EnableUnitFiles {files:?} {runtime} {force}"));
            for file in &files {
                self.check(file)
                    .map_err(|_| SystemdError::NoSuchUnit(file.clone()))?;
            }
            Ok((true, vec![]))
        }
        fn disable_unit_files(
            &self,
            files: Vec<String>,
            runtime: bool,
        ) -> Result<UnitFileChanges, SystemdError> {
            self.note(format!("DisableUnitFiles {files:?} {runtime}"));
            for file in &files {
                self.check(file)
                    .map_err(|_| SystemdError::NoSuchUnit(file.clone()))?;
            }
            Ok(vec![])
        }
        async fn start_unit(
            &self,
            name: String,
            mode: String,
            #[zbus(object_server)] server: &ObjectServer,
        ) -> Result<OwnedObjectPath, SystemdError> {
            self.note(format!("StartUnit {name} {mode}"));
            self.check(&name)
                .map_err(|_| SystemdError::NoSuchUnit(name.clone()))?;
            Self::set_state(server, &name, "active").await;
            Ok(OwnedObjectPath::try_from("/org/freedesktop/systemd1/job/1").unwrap())
        }
        async fn stop_unit(
            &self,
            name: String,
            mode: String,
            #[zbus(object_server)] server: &ObjectServer,
        ) -> Result<OwnedObjectPath, SystemdError> {
            self.note(format!("StopUnit {name} {mode}"));
            self.check(&name)
                .map_err(|_| SystemdError::NoSuchUnit(name.clone()))?;
            Self::set_state(server, &name, "inactive").await;
            Ok(OwnedObjectPath::try_from("/org/freedesktop/systemd1/job/2").unwrap())
        }
        fn restart_unit(&self, name: String, mode: String) -> OwnedObjectPath {
            self.note(format!("RestartUnit {name} {mode}"));
            OwnedObjectPath::try_from("/org/freedesktop/systemd1/job/3").unwrap()
        }
    }

    struct FakeSystemd {
        calls: Calls,
        conn: Connection,
    }

    impl FakeSystemd {
        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }

        /// Cambia el estado como lo haría otro (`systemctl`, Configuración).
        async fn set_state(&self, state: &str) {
            FakeManager::set_state(&self.conn.object_server(), UNIT, state).await;
        }
    }

    async fn serve(
        bus: &PrivateBus,
        active_state: &str,
        load_state: &str,
        need_daemon_reload: bool,
        exists: bool,
    ) -> FakeSystemd {
        let calls: Calls = Arc::default();
        let manager = FakeManager {
            calls: calls.clone(),
            known: if exists { vec![UNIT.into()] } else { vec![] },
        };
        let unit = FakeUnit {
            active_state: active_state.into(),
            load_state: load_state.into(),
            need_daemon_reload,
        };
        let conn = bus
            .connect_serving(|builder| {
                builder
                    .serve_at(MANAGER_PATH, manager)?
                    .serve_at(unit_object_path(UNIT), unit)?
                    .name(SYSTEMD_DEST)
            })
            .await;
        FakeSystemd { calls, conn }
    }

    #[test]
    fn la_ruta_de_la_unidad_es_la_de_systemd() {
        assert_eq!(
            unit_object_path("vasak-nightlight.service"),
            "/org/freedesktop/systemd1/unit/vasak_2dnightlight_2eservice"
        );
        // Un dígito al principio se escapa; en el medio, no.
        assert_eq!(
            unit_object_path("1a2.service"),
            "/org/freedesktop/systemd1/unit/_31a2_2eservice"
        );
        assert_eq!(unit_object_path(""), "/org/freedesktop/systemd1/unit/_");
        assert_eq!(UNIT, "vasak-nightlight.service", "la unidad del plugin");
    }

    #[test]
    fn arrancando_ya_cuenta_como_encendida() {
        for on in ["active", "activating", "reloading"] {
            assert!(is_on(on), "{on}");
        }
        for off in ["inactive", "deactivating", "failed", "maintenance", ""] {
            assert!(!is_on(off), "{off}");
        }
    }

    #[test]
    fn el_estado_se_serializa_para_el_frontend() {
        let json = serde_json::to_value(NightLightState {
            available: true,
            enabled: false,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "available": true, "enabled": false })
        );
    }

    /// Encender habilita y arranca **la unidad del plugin**, en ese orden, sin
    /// `Reload` si systemd ya tiene la unidad al día.
    #[tokio::test]
    async fn encender_habilita_y_arranca_la_unidad() {
        let bus = private_bus!();
        let systemd = serve(&bus, "inactive", "loaded", false, true).await;
        let client = bus.connect().await;

        switch_on(&client, UNIT).await.unwrap();
        assert_eq!(
            systemd.calls(),
            [
                "EnableUnitFiles [\"vasak-nightlight.service\"] false false",
                "StartUnit vasak-nightlight.service replace",
            ]
        );
        assert!(read_enabled(&client, UNIT).await.unwrap());
    }

    /// Con la unidad recién escrita (sin cargar) o cambiada en disco, systemd
    /// tiene que releerla antes de arrancarla: si no, arrancaría la vieja.
    #[tokio::test]
    async fn encender_relee_solo_si_la_unidad_quedo_vieja() {
        let bus = private_bus!();
        let stale = serve(&bus, "inactive", "loaded", true, true).await;
        let client = bus.connect().await;
        switch_on(&client, UNIT).await.unwrap();
        assert_eq!(stale.calls()[0], "Reload");
        drop(stale);

        let bus = private_bus!();
        let new = serve(&bus, "inactive", "not-found", false, true).await;
        let client = bus.connect().await;
        switch_on(&client, UNIT).await.unwrap();
        assert_eq!(new.calls()[0], "Reload");
    }

    /// Apagar deshabilita primero —si no, volvería en el próximo inicio de
    /// sesión— y detiene después.
    #[tokio::test]
    async fn apagar_deshabilita_y_detiene_la_unidad() {
        let bus = private_bus!();
        let systemd = serve(&bus, "active", "loaded", false, true).await;
        let client = bus.connect().await;

        switch_off(&client, UNIT).await.unwrap();
        assert_eq!(
            systemd.calls(),
            [
                "DisableUnitFiles [\"vasak-nightlight.service\"] false",
                "StopUnit vasak-nightlight.service replace",
            ]
        );
        assert!(!read_enabled(&client, UNIT).await.unwrap());
    }

    /// Apagar una luz que nunca se guardó no es un error.
    #[tokio::test]
    async fn apagar_una_unidad_que_no_existe_no_falla() {
        let bus = private_bus!();
        let _systemd = serve(&bus, "inactive", "not-found", false, false).await;
        let client = bus.connect().await;
        switch_off(&client, UNIT).await.unwrap();
        // Pero encenderla sí: no hay qué arrancar.
        assert!(switch_on(&client, UNIT).await.is_err());
    }

    /// Una configuración nueva reinicia `wlsunset` sólo si está corriendo.
    #[tokio::test]
    async fn aplicar_la_configuracion_reinicia_solo_si_esta_encendida() {
        let bus = private_bus!();
        let on = serve(&bus, "active", "loaded", true, true).await;
        let client = bus.connect().await;
        assert!(apply_config(&client, UNIT).await.unwrap());
        assert_eq!(
            on.calls(),
            ["Reload", "RestartUnit vasak-nightlight.service replace"]
        );
        drop(on);

        let bus = private_bus!();
        let off = serve(&bus, "inactive", "loaded", true, true).await;
        let client = bus.connect().await;
        assert!(!apply_config(&client, UNIT).await.unwrap());
        assert!(off.calls().is_empty(), "apagada no se toca");
    }

    async fn recv(rx: &mut tokio::sync::mpsc::UnboundedReceiver<bool>) -> bool {
        tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("llega el cambio")
            .expect("el canal sigue abierto")
    }

    /// El seguimiento se suscribe, lee el estado y se entera de un cambio
    /// hecho por otro (`systemctl`, Configuración) por la señal, sin
    /// preguntar.
    #[tokio::test]
    async fn el_seguimiento_se_entera_por_la_senal() {
        let bus = private_bus!();
        let systemd = serve(&bus, "inactive", "loaded", false, true).await;
        let follower = bus.connect().await;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let task = tokio::spawn(async move {
            let _ = follow(&follower, UNIT, move |enabled| {
                let _ = tx.send(enabled);
            })
            .await;
        });

        assert!(!recv(&mut rx).await, "lee el estado al empezar");
        assert_eq!(
            systemd.calls(),
            ["Subscribe"],
            "sin Subscribe no hay señales"
        );

        systemd.set_state("activating").await;
        assert!(recv(&mut rx).await, "el cambio llega por PropertiesChanged");
        systemd.set_state("failed").await;
        assert!(!recv(&mut rx).await, "y si wlsunset no arranca, se apaga");

        task.abort();
    }

    /// Una señal de otra unidad no cambia el mosaico.
    #[tokio::test]
    async fn la_senal_de_otra_unidad_no_cuenta() {
        let bus = private_bus!();
        let systemd = serve(&bus, "inactive", "loaded", false, true).await;
        let other = "otra.service";
        systemd
            .conn
            .object_server()
            .at(
                unit_object_path(other),
                FakeUnit {
                    active_state: "inactive".into(),
                    load_state: "loaded".into(),
                    need_daemon_reload: false,
                },
            )
            .await
            .unwrap();
        let follower = bus.connect().await;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            let _ = follow(&follower, UNIT, move |enabled| {
                let _ = tx.send(enabled);
            })
            .await;
        });
        assert!(!recv(&mut rx).await);

        FakeManager::set_state(&systemd.conn.object_server(), other, "active").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(rx.try_recv().is_err(), "otra unidad no es la luz");

        task.abort();
    }

    #[tokio::test]
    async fn desuscribirse_se_lo_dice_a_systemd() {
        let bus = private_bus!();
        let systemd = serve(&bus, "inactive", "loaded", false, true).await;
        let client = bus.connect().await;
        unsubscribe(&client).await;
        assert_eq!(systemd.calls(), ["Unsubscribe"]);
    }

    /// Contra el systemd de esta sesión, con una unidad de prueba que corre
    /// `sleep` —no `wlsunset`: no cambia la pantalla— y se borra al final.
    /// Comprueba las firmas de verdad de los métodos y que la señal llega.
    ///
    /// `cargo test night_light -- --ignored`
    #[tokio::test]
    #[ignore = "usa el systemd de la sesión"]
    async fn contra_el_systemd_de_la_sesion() {
        let unit = format!("vasak-night-light-probe-{}.service", std::process::id());
        let dir = dirs::config_dir().unwrap().join("systemd/user");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(&unit);
        std::fs::write(
            &file,
            "[Unit]\nDescription=prueba de vasak-desktop\n[Service]\nExecStart=/usr/bin/sleep infinity\n[Install]\nWantedBy=default.target\n",
        )
        .unwrap();

        let conn = Connection::session().await.unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let follower = conn.clone();
        let watched = unit.clone();
        let task = tokio::spawn(async move {
            follow(&follower, &watched, move |on| {
                let _ = tx.send(on);
            })
            .await
            .unwrap();
        });
        assert!(!recv(&mut rx).await);

        let started = std::time::Instant::now();
        switch_on(&conn, &unit).await.unwrap();
        // El `Reload` de la unidad recién escrita también avisa, con el
        // estado de antes: se espera hasta el encendido.
        while !recv(&mut rx).await {}
        eprintln!("encender: {:?}", started.elapsed());
        assert!(dir.join("default.target.wants").join(&unit).exists());
        assert!(apply_config(&conn, &unit).await.unwrap());

        switch_off(&conn, &unit).await.unwrap();
        let mut on = true;
        while on {
            on = recv(&mut rx).await;
        }
        assert!(!dir.join("default.target.wants").join(&unit).exists());
        task.abort();
        unsubscribe(&conn).await;

        std::fs::remove_file(&file).unwrap();
        manager(&conn)
            .await
            .unwrap()
            .call::<_, _, ()>("Reload", &())
            .await
            .unwrap();
    }
}
