//! «No molestar», del lado del escritorio (vasak-desktop#177).
//!
//! El modo vive en el servidor de notificaciones (`vasak-flare-daemon`), que es
//! quien crea los carteles y por eso quien los corta. Este módulo es su cliente:
//! lo pone, lo lee y lo sigue.
//!
//! - **Una copia en memoria** ([`is_enabled`], [`state`]) que se actualiza con
//!   las señales del demonio. Preguntar no cruza el bus.
//! - **Sin sondeos:** el demonio avisa cada cambio por `PropertiesChanged`, y si
//!   se reinicia, `NameOwnerChanged` hace releer el valor. Cada cambio sale al
//!   frontend como el evento [`CHANGED_EVENT`].
//! - **Para el modo juego (#181):** [`set_enabled`] devuelve el estado anterior
//!   en la misma llamada, así que restaurarlo al salir es
//!   `set_enabled(anterior)`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use zbus::fdo::{DBusProxy, PropertiesProxy};
use zbus::names::InterfaceName;
use zbus::{Connection, Proxy};

use crate::logger::{log_error, log_info};

const FLARE_DEST: &str = "org.vasak.Notifications";
const FLARE_PATH: &str = "/org/vasak/Notifications";
/// La interfaz del modo en el demonio. Es contrato con `vasak-flare-daemon`
/// 0.6.0: su prueba del bus usa este mismo nombre.
const DND_IFACE: &str = "org.vasak.Notifications.DoNotDisturb";

/// El evento que reciben las ventanas con cada cambio, con un
/// [`DoNotDisturbState`] adentro.
pub const CHANGED_EVENT: &str = "do-not-disturb-changed";

/// Lo que se sabe del modo.
///
/// `available` es falso mientras el demonio no contesta o es anterior a la
/// 0.6.0: el mosaico se ve **no disponible**, nunca roto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DoNotDisturbState {
    pub available: bool,
    pub enabled: bool,
}

/// La copia en memoria. Aparte de la global para que cada prueba tenga la suya.
#[derive(Debug, Default)]
pub struct Mirror {
    available: AtomicBool,
    enabled: AtomicBool,
}

impl Mirror {
    pub const fn new() -> Self {
        Self {
            available: AtomicBool::new(false),
            enabled: AtomicBool::new(false),
        }
    }

    pub fn state(&self) -> DoNotDisturbState {
        DoNotDisturbState {
            available: self.available.load(Ordering::Relaxed),
            enabled: self.enabled.load(Ordering::Relaxed),
        }
    }

    /// Guarda lo que se supo y dice si cambió algo, para no emitir de más.
    pub fn apply(&self, next: DoNotDisturbState) -> bool {
        let was_available = self.available.swap(next.available, Ordering::AcqRel);
        let was_enabled = self.enabled.swap(next.enabled, Ordering::AcqRel);
        was_available != next.available || was_enabled != next.enabled
    }
}

static MIRROR: Mirror = Mirror::new();

/// El estado conocido, sin cruzar el bus.
pub fn state() -> DoNotDisturbState {
    MIRROR.state()
}

/// Si el modo está puesto, sin cruzar el bus.
pub fn is_enabled() -> bool {
    MIRROR.state().enabled
}

async fn dnd_proxy(conn: &Connection) -> zbus::Result<Proxy<'static>> {
    zbus::proxy::Builder::new(conn)
        .destination(FLARE_DEST)?
        .path(FLARE_PATH)?
        .interface(DND_IFACE)?
        // Sin caché: la copia es `Mirror`, y una caché de zbus pediría todas las
        // propiedades al crearse por las dudas.
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
}

/// Lee el modo del demonio. Sin demonio, o con uno viejo, no disponible.
async fn fetch(conn: &Connection) -> DoNotDisturbState {
    let read = async { dnd_proxy(conn).await?.get_property::<bool>("Enabled").await };
    match read.await {
        Ok(enabled) => DoNotDisturbState {
            available: true,
            enabled,
        },
        Err(_) => DoNotDisturbState {
            available: false,
            enabled: false,
        },
    }
}

/// Pone el modo en el demonio y devuelve el que había.
///
/// La copia en memoria se actualiza al toque; el evento lo dispara la señal
/// del demonio, que llega a todas las ventanas por el mismo camino.
pub async fn set_enabled_on(
    conn: &Connection,
    mirror: &Mirror,
    enabled: bool,
) -> Result<bool, String> {
    let proxy = dnd_proxy(conn).await.map_err(|e| e.to_string())?;
    let previous: bool = proxy
        .call("SetEnabled", &(enabled,))
        .await
        .map_err(|e| format!("el demonio de notificaciones no aceptó «No molestar»: {e}"))?;
    mirror.apply(DoNotDisturbState {
        available: true,
        enabled,
    });
    Ok(previous)
}

/// Pone el modo y devuelve el anterior. Es la entrada para el modo juego:
///
/// ```ignore
/// let previous = do_not_disturb::set_enabled(true).await?;
/// // … al salir del juego …
/// do_not_disturb::set_enabled(previous).await?;
/// ```
pub async fn set_enabled(enabled: bool) -> Result<bool, String> {
    let conn = crate::notifications::connection().await?;
    set_enabled_on(&conn, &MIRROR, enabled).await
}

/// Sigue al demonio hasta que la conexión se corta: lee el valor, y lo vuelve
/// a leer cuando cambia (`PropertiesChanged`) o cuando el demonio se reinicia
/// (`NameOwnerChanged`). `on_change` se llama sólo si el estado cambió.
pub async fn follow(
    conn: &Connection,
    mirror: &Mirror,
    on_change: impl Fn(DoNotDisturbState),
) -> zbus::Result<()> {
    let properties = PropertiesProxy::builder(conn)
        .destination(FLARE_DEST)?
        .path(FLARE_PATH)?
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await?;
    let mut changes = properties
        .receive_properties_changed_with_args(&[(0, DND_IFACE)])
        .await?;
    let mut owners = DBusProxy::new(conn)
        .await?
        .receive_name_owner_changed_with_args(&[(0, FLARE_DEST)])
        .await?;

    // Lo primero, después de suscribirse: así un cambio entre la lectura y la
    // suscripción no se pierde.
    if mirror.apply(fetch(conn).await) {
        on_change(mirror.state());
    }

    let iface = InterfaceName::from_static_str_unchecked(DND_IFACE);
    loop {
        let next = tokio::select! {
            change = changes.next() => {
                let Some(change) = change else { return Ok(()) };
                let Ok(args) = change.args() else { continue };
                if args.interface_name() != &iface {
                    continue;
                }
                match args.changed_properties().get("Enabled").map(bool::try_from) {
                    Some(Ok(enabled)) => DoNotDisturbState { available: true, enabled },
                    // Invalidada o con otro tipo: se pregunta.
                    _ => fetch(conn).await,
                }
            }
            owner = owners.next() => {
                if owner.is_none() {
                    return Ok(());
                }
                // Se fue, o volvió con el valor que guardó: se pregunta igual.
                fetch(conn).await
            }
        };
        if mirror.apply(next) {
            on_change(mirror.state());
        }
    }
}

/// Arranca el seguimiento, reconectando con espera creciente si el bus falla.
pub async fn start(app: AppHandle) {
    tokio::spawn(async move {
        let mut delay = Duration::from_secs(1);
        loop {
            let emit = |state: DoNotDisturbState| {
                log_info(&format!(
                    "«No molestar»: {}",
                    if state.enabled { "puesto" } else { "quitado" }
                ));
                let _ = app.emit(CHANGED_EVENT, state);
            };
            let result = match crate::notifications::connection().await {
                Ok(conn) => follow(&conn, &MIRROR, emit)
                    .await
                    .map_err(|e| e.to_string()),
                Err(e) => Err(e),
            };
            match result {
                Ok(()) => delay = Duration::from_secs(1),
                Err(e) => {
                    log_error(&format!("No se pudo seguir «No molestar»: {e}"));
                    delay = (delay * 2).min(Duration::from_secs(30));
                }
            }
            tokio::time::sleep(delay).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tray::bus_tests::PrivateBus;
    use std::sync::Arc;
    use zbus::interface;
    use zbus::object_server::SignalContext;

    /// Un demonio de notificaciones de mentira con la interfaz de la 0.6.0.
    struct FakeFlare {
        enabled: Arc<AtomicBool>,
    }

    #[interface(name = "org.vasak.Notifications.DoNotDisturb")]
    impl FakeFlare {
        #[zbus(property)]
        async fn enabled(&self) -> bool {
            self.enabled.load(Ordering::SeqCst)
        }

        async fn set_enabled(
            &self,
            enabled: bool,
            #[zbus(signal_context)] ctxt: SignalContext<'_>,
        ) -> bool {
            let previous = self.enabled.swap(enabled, Ordering::SeqCst);
            if previous != enabled {
                let _ = self.enabled_changed(&ctxt).await;
            }
            previous
        }
    }

    async fn recv(
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<DoNotDisturbState>,
    ) -> DoNotDisturbState {
        tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("llega el cambio")
            .expect("el canal sigue abierto")
    }

    async fn serve_fake(bus: &PrivateBus, enabled: bool) -> Connection {
        bus.connect_serving(|builder| {
            builder
                .serve_at(
                    FLARE_PATH,
                    FakeFlare {
                        enabled: Arc::new(AtomicBool::new(enabled)),
                    },
                )?
                .name(FLARE_DEST)
        })
        .await
    }

    #[test]
    fn el_estado_se_serializa_para_el_frontend() {
        let json = serde_json::to_value(DoNotDisturbState {
            available: true,
            enabled: false,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "available": true, "enabled": false })
        );
    }

    #[test]
    fn la_copia_dice_si_algo_cambio() {
        let mirror = Mirror::new();
        let on = DoNotDisturbState {
            available: true,
            enabled: true,
        };
        assert!(mirror.apply(on));
        assert!(!mirror.apply(on), "lo mismo dos veces no es un cambio");
        assert_eq!(mirror.state(), on);
    }

    /// `set_enabled` devuelve el anterior y deja la copia al día: es lo que
    /// usa el modo juego para restaurar el modo al salir.
    #[tokio::test]
    async fn poner_el_modo_devuelve_el_anterior() {
        let bus = crate::tray::bus_tests::private_bus!();
        let _flare = serve_fake(&bus, false).await;
        let client = bus.connect().await;
        let mirror = Mirror::new();

        assert!(!set_enabled_on(&client, &mirror, true).await.unwrap());
        assert!(mirror.state().enabled);
        assert!(set_enabled_on(&client, &mirror, false).await.unwrap());
        assert!(!mirror.state().enabled);
    }

    /// El seguimiento lee el valor guardado al arrancar y se entera de un
    /// cambio hecho por otro (otra ventana, el modo juego) por la señal, sin
    /// preguntar.
    #[tokio::test]
    async fn el_seguimiento_se_entera_por_la_senal() {
        let bus = crate::tray::bus_tests::private_bus!();
        let _flare = serve_fake(&bus, true).await;
        let follower = bus.connect().await;
        let other = bus.connect().await;
        let mirror = Arc::new(Mirror::new());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let task = {
            let mirror = mirror.clone();
            tokio::spawn(async move {
                let _ = follow(&follower, &mirror, move |state| {
                    let _ = tx.send(state);
                })
                .await;
            })
        };

        let first = recv(&mut rx).await;
        assert_eq!(
            first,
            DoNotDisturbState {
                available: true,
                enabled: true
            },
            "lee lo que quedó de la sesión anterior"
        );

        set_enabled_on(&other, &Mirror::new(), false).await.unwrap();
        let second = recv(&mut rx).await;
        assert!(!second.enabled, "el cambio llega por PropertiesChanged");
        assert!(!mirror.state().enabled);

        task.abort();
    }

    /// Sin demonio (o con uno anterior a la 0.6.0) el modo no está disponible.
    #[tokio::test]
    async fn sin_demonio_no_esta_disponible() {
        let bus = crate::tray::bus_tests::private_bus!();
        let client = bus.connect().await;
        assert_eq!(
            fetch(&client).await,
            DoNotDisturbState {
                available: false,
                enabled: false
            }
        );
        assert!(set_enabled_on(&client, &Mirror::new(), true).await.is_err());
    }
}
