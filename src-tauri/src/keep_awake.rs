//! «Mantener despierto», del lado del escritorio (vasak-desktop#179).
//!
//! Mientras está puesto, el escritorio tiene tomado un **inhibidor de logind**:
//! `org.freedesktop.login1.Manager.Inhibit` devuelve un descriptor y el
//! inhibidor dura lo que dura ese descriptor abierto. Soltarlo lo apaga. No hay
//! nada más que mantener: ni sondeos, ni temporizadores, ni subprocesos.
//!
//! # Qué se inhibe, y por qué sólo `idle`
//!
//! Se pide `idle` en modo `block`, que es lo que cubre las dos cosas que el modo
//! promete:
//!
//! - **el bloqueo por inactividad.** `vasak-idle.service` es `swayidle` 1.9, que
//!   sigue la propiedad `BlockInhibited` de logind por `PropertiesChanged`: al
//!   aparecer `idle` apaga sus temporizadores —el bloqueo, apagar la pantalla,
//!   pausar el fondo— y al irse los vuelve a poner. Lo hace **sólo si está
//!   conectado a logind**, cosa que pasa cuando la línea lleva `before-sleep`,
//!   `after-resume`, `lock`, `unlock` o `idlehint`. La unidad del paquete
//!   (`vasak-desktop-settings`) lleva `before-sleep`;
//! - **la suspensión por inactividad.** La única que hay es `IdleAction` de
//!   logind, y la frena el mismo inhibidor `idle`.
//!
//! `sleep` en modo `block` no se pide a propósito: no frena nada que pase solo
//! —la tapa ignora los inhibidores por omisión (`LidSwitchIgnoreInhibited`) y lo
//! que suspende como root los saltea— y en cambio sí frena **la suspensión que
//! pide la misma persona**: con un `sleep` tomado, `CanSuspend` contesta
//! `inhibited` (medido en systemd 262) y «Suspender» del menú de sesión pasaría
//! a pedir la contraseña de administrador.
//!
//! # Si el escritorio se cae
//!
//! El descriptor es del proceso: si el escritorio muere, el núcleo lo cierra y
//! logind suelta el inhibidor solo. No queda nada que restaurar, y por eso el
//! modo **no se recuerda entre sesiones**: siempre arranca apagado.
//!
//! # Eficiencia
//!
//! Preguntar si está puesto es leer un atómico. Saber si está disponible es una
//! llamada al bus cuando se monta el mosaico; con el modo apagado no corre nada.

use std::os::fd::OwnedFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
use zbus::Connection;

use crate::dbus_pool::DbusPool;
use crate::logger::{log_error, log_info};

pub(crate) const LOGIND_DEST: &str = "org.freedesktop.login1";
pub(crate) const LOGIND_PATH: &str = "/org/freedesktop/login1";
pub(crate) const LOGIND_IFACE: &str = "org.freedesktop.login1.Manager";

/// Lo que se inhibe. Ver el encabezado del módulo: `sleep` no va.
pub(crate) const WHAT: &str = "idle";
/// Quién lo pide, como lo muestra `systemd-inhibit --list`.
pub(crate) const WHO: &str = "VasakOS";
/// `block`: `delay` sólo vale para `sleep` y `shutdown`, y `block-weak` no lo
/// ve `swayidle`, que sólo lee `BlockInhibited`.
pub(crate) const MODE: &str = "block";

/// La clave del motivo en el catálogo, y lo que se usa si no está.
const REASON_KEY: &str = "components.ControlCenterTiles.keepAwakeReason";
const REASON_FALLBACK: &str = "Mantener despierto está activado";

/// El evento que reciben las ventanas con cada cambio, con un
/// [`KeepAwakeState`] adentro.
pub const CHANGED_EVENT: &str = "keep-awake-changed";

/// Lo que se sabe del modo. `available` es falso sin logind en el bus del
/// sistema: el mosaico se ve **no disponible**, nunca roto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct KeepAwakeState {
    pub available: bool,
    pub enabled: bool,
}

/// El inhibidor tomado, si lo hay. Aparte de la global para que cada prueba
/// tenga el suyo.
#[derive(Debug, Default)]
pub struct KeepAwake {
    /// El descriptor que devolvió logind. Soltarlo es apagar el modo.
    ///
    /// Con candado de tokio porque se sostiene durante la llamada al bus: dos
    /// toques seguidos no pueden pedir dos inhibidores.
    held: Mutex<Option<OwnedFd>>,
    /// Copia de `held.is_some()` para leerla sin esperar el candado.
    enabled: AtomicBool,
}

impl KeepAwake {
    pub const fn new() -> Self {
        Self {
            held: Mutex::const_new(None),
            enabled: AtomicBool::new(false),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Lo pone o lo quita y devuelve si estaba puesto.
    ///
    /// Ponerlo con el modo ya puesto no pide otro inhibidor; quitarlo cierra el
    /// descriptor, que es lo que le dice a logind que lo suelte. `conn` es el bus
    /// del sistema y sólo se usa para ponerlo: quitarlo no cruza el bus.
    pub async fn set_on(
        &self,
        conn: Option<&Connection>,
        enabled: bool,
        why: &str,
    ) -> Result<bool, String> {
        let mut held = self.held.lock().await;
        let previous = held.is_some();
        if enabled == previous {
            return Ok(previous);
        }
        if enabled {
            let conn = conn.ok_or_else(|| "no hay conexión con el bus del sistema".to_string())?;
            *held = Some(inhibit(conn, why).await?);
        } else {
            // Al soltarse se cierra, y logind lo saca de la lista.
            drop(held.take());
        }
        self.enabled.store(enabled, Ordering::Release);
        Ok(previous)
    }

    /// El estado, con `available` según si logind está en `conn`.
    pub async fn state_on(&self, conn: Option<&Connection>) -> KeepAwakeState {
        // Con el inhibidor tomado logind está, sin preguntar.
        if self.is_enabled() {
            return KeepAwakeState {
                available: true,
                enabled: true,
            };
        }
        let available = match conn {
            Some(conn) => crate::dbus_pool::name_on_bus(conn, LOGIND_DEST).await,
            None => false,
        };
        KeepAwakeState {
            available,
            enabled: false,
        }
    }
}

/// Pide el inhibidor a logind y devuelve su descriptor.
async fn inhibit(conn: &Connection, why: &str) -> Result<OwnedFd, String> {
    let reply = conn
        .call_method(
            Some(LOGIND_DEST),
            LOGIND_PATH,
            Some(LOGIND_IFACE),
            "Inhibit",
            &(WHAT, WHO, why, MODE),
        )
        .await
        .map_err(|e| format!("logind no dio el inhibidor: {e}"))?;
    let fd: zbus::zvariant::OwnedFd = reply
        .body()
        .deserialize()
        .map_err(|e| format!("logind contestó algo que no es un descriptor: {e}"))?;
    Ok(fd.into())
}

static KEEP_AWAKE: KeepAwake = KeepAwake::new();

/// Para publicar los cambios a las ventanas. Se pone en la primera llamada.
static APP: OnceLock<AppHandle> = OnceLock::new();

async fn system_bus(app: &AppHandle) -> Option<Connection> {
    app.try_state::<DbusPool>()?.system().await
}

fn reason(app: &AppHandle) -> String {
    use tauri_plugin_i18n_vsk::PluginI18nExt;
    app.try_state::<tauri_plugin_i18n_vsk::PluginI18n<tauri::Wry>>()
        .and_then(|_| app.i18n().translate(REASON_KEY).map(str::to_string))
        .unwrap_or_else(|| REASON_FALLBACK.to_string())
}

/// El estado del modo. Ver [`KeepAwake::state_on`].
pub async fn state(app: &AppHandle) -> KeepAwakeState {
    let conn = system_bus(app).await;
    KEEP_AWAKE.state_on(conn.as_ref()).await
}

/// Si está puesto, sin cruzar el bus.
pub fn is_enabled() -> bool {
    KEEP_AWAKE.is_enabled()
}

/// Lo pone o lo quita, avisa a las ventanas si cambió y devuelve el anterior.
pub async fn set_enabled(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    let _ = APP.set(app.clone());
    let conn = system_bus(app).await;
    let previous = KEEP_AWAKE
        .set_on(conn.as_ref(), enabled, &reason(app))
        .await
        .inspect_err(|e| log_error(&format!("«Mantener despierto»: {e}")))?;
    if previous != enabled {
        log_info(&format!(
            "«Mantener despierto»: {}",
            if enabled { "puesto" } else { "quitado" }
        ));
        if let Some(app) = APP.get() {
            let _ = app.emit(
                CHANGED_EVENT,
                KeepAwakeState {
                    available: true,
                    enabled,
                },
            );
        }
    }
    Ok(previous)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tray::bus_tests::{private_bus, PrivateBus};
    use std::io::Read;
    use std::os::unix::net::UnixStream;
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::Duration;
    use zbus::interface;

    /// Lo que se pidió a logind: los argumentos, y del otro lado de cada
    /// descriptor entregado, el extremo que se queda logind.
    #[derive(Default)]
    struct Requests {
        args: Vec<(String, String, String, String)>,
        kept: Vec<UnixStream>,
    }

    /// Un logind de mentira: `Inhibit` entrega un extremo de un par de sockets
    /// y se queda el otro, como hace el de verdad con su FIFO. Cuando el que
    /// lo pidió cierra el suyo, el extremo guardado lee fin de archivo.
    struct FakeLogind {
        requests: Arc<StdMutex<Requests>>,
    }

    #[interface(name = "org.freedesktop.login1.Manager")]
    impl FakeLogind {
        async fn inhibit(
            &self,
            what: String,
            who: String,
            why: String,
            mode: String,
        ) -> zbus::fdo::Result<zbus::zvariant::OwnedFd> {
            let (ours, theirs) =
                UnixStream::pair().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
            let mut requests = self.requests.lock().unwrap();
            requests.args.push((what, who, why, mode));
            requests.kept.push(ours);
            Ok(OwnedFd::from(theirs).into())
        }
    }

    async fn serve_fake(bus: &PrivateBus) -> (Connection, Arc<StdMutex<Requests>>) {
        let requests = Arc::new(StdMutex::new(Requests::default()));
        let conn = bus
            .connect_serving(|builder| {
                builder
                    .serve_at(
                        LOGIND_PATH,
                        FakeLogind {
                            requests: requests.clone(),
                        },
                    )?
                    .name(LOGIND_DEST)
            })
            .await;
        (conn, requests)
    }

    /// Si el que pidió el inhibidor ya lo cerró: el extremo de logind lee fin
    /// de archivo. Si sigue abierto, la lectura vence sin datos.
    fn released(kept: &UnixStream) -> bool {
        let mut kept = kept.try_clone().unwrap();
        kept.set_read_timeout(Some(Duration::from_millis(300)))
            .unwrap();
        let mut buf = [0u8; 1];
        matches!(kept.read(&mut buf), Ok(0))
    }

    #[test]
    fn el_estado_se_serializa_para_el_frontend() {
        let json = serde_json::to_value(KeepAwakeState {
            available: true,
            enabled: false,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "available": true, "enabled": false })
        );
    }

    /// Lo que va a logind: sólo `idle`, en modo `block`, a nombre de VasakOS.
    #[test]
    fn pide_idle_en_modo_block() {
        assert_eq!(WHAT, "idle", "con `sleep` el menú de sesión no suspende");
        assert_eq!(MODE, "block", "swayidle sólo lee BlockInhibited");
        assert_eq!(WHO, "VasakOS");
    }

    /// Activar toma el inhibidor y desactivar lo suelta: el extremo de logind
    /// ve el cierre.
    #[tokio::test]
    async fn activar_toma_el_inhibidor_y_desactivar_lo_suelta() {
        let bus = private_bus!();
        let (_logind, requests) = serve_fake(&bus).await;
        let client = bus.connect().await;
        let keep = KeepAwake::new();

        let previous = keep.set_on(Some(&client), true, "un motivo").await.unwrap();
        assert!(!previous);
        assert!(keep.is_enabled());
        {
            let requests = requests.lock().unwrap();
            assert_eq!(
                requests.args,
                vec![(
                    "idle".to_string(),
                    "VasakOS".to_string(),
                    "un motivo".to_string(),
                    "block".to_string()
                )]
            );
            assert!(
                !released(&requests.kept[0]),
                "mientras está puesto, el descriptor sigue abierto"
            );
        }

        let previous = keep.set_on(Some(&client), false, "").await.unwrap();
        assert!(previous);
        assert!(!keep.is_enabled());
        let requests = requests.lock().unwrap();
        assert!(released(&requests.kept[0]), "quitarlo cierra el descriptor");
    }

    /// Activar dos veces —también dos toques a la vez— no toma dos.
    #[tokio::test]
    async fn activar_dos_veces_no_toma_dos() {
        let bus = private_bus!();
        let (_logind, requests) = serve_fake(&bus).await;
        let client = bus.connect().await;
        let keep = Arc::new(KeepAwake::new());

        let (a, b) = tokio::join!(
            keep.set_on(Some(&client), true, "a"),
            keep.set_on(Some(&client), true, "b"),
        );
        assert_eq!(
            [a.unwrap(), b.unwrap()].iter().filter(|p| !**p).count(),
            1,
            "uno lo puso y el otro lo encontró puesto"
        );
        keep.set_on(Some(&client), true, "c").await.unwrap();
        assert_eq!(requests.lock().unwrap().args.len(), 1);

        // Quitarlo dos veces tampoco hace nada raro.
        assert!(keep.set_on(Some(&client), false, "").await.unwrap());
        assert!(!keep.set_on(Some(&client), false, "").await.unwrap());
        assert!(released(&requests.lock().unwrap().kept[0]));
    }

    /// Con logind en el bus está disponible; puesto, se ve encendido sin
    /// preguntar.
    #[tokio::test]
    async fn con_logind_esta_disponible() {
        let bus = private_bus!();
        let (_logind, _) = serve_fake(&bus).await;
        let client = bus.connect().await;
        let keep = KeepAwake::new();

        assert_eq!(
            keep.state_on(Some(&client)).await,
            KeepAwakeState {
                available: true,
                enabled: false
            }
        );
        keep.set_on(Some(&client), true, "").await.unwrap();
        assert_eq!(
            keep.state_on(None).await,
            KeepAwakeState {
                available: true,
                enabled: true
            }
        );
    }

    /// Sin logind —o sin bus del sistema— no está disponible y ponerlo falla
    /// sin quedar a medias.
    #[tokio::test]
    async fn sin_logind_no_esta_disponible() {
        let bus = private_bus!();
        let client = bus.connect().await;
        let keep = KeepAwake::new();
        let unavailable = KeepAwakeState {
            available: false,
            enabled: false,
        };

        assert_eq!(keep.state_on(Some(&client)).await, unavailable);
        assert_eq!(keep.state_on(None).await, unavailable);
        assert!(keep.set_on(Some(&client), true, "").await.is_err());
        assert!(keep.set_on(None, true, "").await.is_err());
        assert!(!keep.is_enabled());
    }

    /// Contra el logind de verdad: con el modo puesto, `systemd-inhibit --list`
    /// lo muestra, y al quitarlo desaparece. No corre con `cargo test` porque
    /// toca el bus del sistema: `cargo test -- --ignored con_el_logind_de_verdad`.
    #[tokio::test]
    #[ignore = "toca el logind de la máquina"]
    async fn con_el_logind_de_verdad() {
        let conn = Connection::system().await.expect("bus del sistema");
        let keep = KeepAwake::new();
        let why = format!("prueba de vasak-desktop {}", std::process::id());
        let listed = |why: &str| {
            let out = std::process::Command::new("systemd-inhibit")
                .args(["--list", "--no-pager", "--no-legend", "--what=idle"])
                .output()
                .expect("systemd-inhibit");
            String::from_utf8_lossy(&out.stdout).contains(why)
        };

        keep.set_on(Some(&conn), true, &why).await.unwrap();
        assert!(listed(&why), "systemd-inhibit --list lo muestra");
        keep.set_on(Some(&conn), false, &why).await.unwrap();
        // logind se entera del cierre por su bucle; se le da un momento.
        let mut gone = false;
        for _ in 0..20 {
            if !listed(&why) {
                gone = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(gone, "al quitarlo, logind lo suelta");
    }
}
