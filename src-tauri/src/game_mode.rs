//! El modo juego (vasak-desktop#181).
//!
//! Mientras está puesto aplica una lista de **acciones**, y al salir deja cada
//! cosa como estaba. Qué acciones se aplican lo dice la sección `game_mode` de
//! `vasak.conf` (la edita Configuración, Vasak-OS/vasak-settings#156).
//!
//! # El registro de acciones
//!
//! Cada acción es una entrada de [`REGISTRY`] —su clave en la configuración y
//! su valor por omisión— y tres pasos:
//!
//! - **entrar** ([`Action::enter`]): la aplica y devuelve lo necesario para
//!   deshacerla, un [`Undo`]. Si no había nada que cambiar, no devuelve nada;
//! - **salir** ([`Undo::exit`]): la deshace en la misma sesión;
//! - **restaurar** ([`Undo::restore`]): la deshace después de una caída del
//!   escritorio, con lo que quedó en el archivo de recuperación.
//!
//! Sumar una acción es sumar una entrada, su variante y su clave.
//!
//! | clave | qué hace | por omisión |
//! |---|---|---|
//! | `do_not_disturb` | pone «No molestar»; al salir lo quita, salvo que alguien lo haya tocado durante el juego | encendida |
//! | `disable_animations` | apaga las animaciones de Wayfire por su IPC, **en memoria**, sin escribir `wayfire.ini` | encendida |
//! | `gamemode` | registra el escritorio en `gamemoded` por D-Bus, si está instalado; si no, se saltea | encendida |
//!
//! # Eficiencia
//!
//! - La configuración se lee **una vez**, al entrar. No se vigila mientras dura:
//!   un cambio vale para la próxima vez.
//! - Preguntar si el modo está puesto es leer un atómico.
//! - Nada corre mientras el modo está puesto: ni sondeos ni subprocesos. Todo va
//!   por D-Bus y por el socket de Wayfire.
//!
//! # Las animaciones
//!
//! `animate/open_animation`, `animate/close_animation` y
//! `animate/minimize_animation` pasan a `none` con `wayfire/set-config-options`,
//! que cambia la opción en el compositor sin tocar el archivo. Se anota el valor
//! que tenía cada una. Al salir, una opción que **ya no** está en `none` la cambió
//! alguien más (Configuración reescribió `wayfire.ini`, o Wayfire se reinició y
//! la volvió a leer de ahí): ésa no se toca. Por eso salir y restaurar después
//! de una caída son lo mismo.
//!
//! No se descarga el complemento `animate` de `core/plugins`: sacarlo de la
//! lista en memoria lo destruye y lo vuelve a crear, y reescribir esa lista
//! entera para quitar uno es justo lo que no se quiere hacer sin escribir el
//! archivo.
//!
//! # `gamemoded` y el pid que se registra
//!
//! `gamemoded` lleva la cuenta de **procesos** registrados y se apaga cuando no
//! queda ninguno; su recolector revisa cada pocos segundos (`reaper_freq`, 5 por
//! omisión) si siguen vivos. El modo juego es global, no de un juego, así que
//! se registra **el propio escritorio** (`std::process::id()`):
//!
//! - vive toda la sesión, así que el modo no se cae solo mientras dura;
//! - si el escritorio se cae, el recolector de `gamemoded` lo da de baja por su
//!   cuenta. No hay nada que restaurar de este lado;
//! - lo que `gamemoded` hace por proceso (`renice`, `ioprio`, fijar núcleos) se
//!   aplica al escritorio y no al juego. Con su configuración por omisión
//!   `renice` está apagado; lo que importa del modo global —el gobernador de la
//!   CPU, la GPU, inhibir el salvapantallas— no depende del pid.
//!
//! Que `gamemoded` esté se sabe preguntando al bus si el nombre es activable o
//! tiene dueño, una llamada; sin él la acción se saltea sin error.
//!
//! # Si el escritorio se cae
//!
//! Lo aplicado se anota en `$XDG_RUNTIME_DIR/vasak-desktop-game-mode.json`
//! después de cada acción. Al arrancar, si el archivo está, se restaura lo que
//! dice y se borra. Ese directorio se vacía al cerrar la sesión, así que el
//! modo no se recuerda entre sesiones. Y como el servidor de notificaciones sí
//! guarda «No molestar» entre sesiones, al recibir `SIGTERM` o `SIGINT` con el
//! modo puesto se sale de él antes de terminar.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::logger::{log_error, log_info, log_warning};

/// El evento que reciben las ventanas con cada cambio, con el estado nuevo.
pub const CHANGED_EVENT: &str = "game-mode-changed";

/// La sección de `vasak.conf`.
pub const CONFIG_SECTION: &str = "game_mode";

/// El archivo de recuperación, dentro de `$XDG_RUNTIME_DIR`.
const RECOVERY_FILE: &str = "vasak-desktop-game-mode.json";

/// Las opciones de Wayfire que apaga `disable_animations`.
pub const ANIMATION_OPTIONS: [&str; 3] = [
    "animate/open_animation",
    "animate/close_animation",
    "animate/minimize_animation",
];

/// El valor que las apaga. Las tres lo aceptan (ver `animate.xml`).
pub const NO_ANIMATION: &str = "none";

const GAMEMODE_DEST: &str = "com.feralinteractive.GameMode";
const GAMEMODE_PATH: &str = "/com/feralinteractive/GameMode";
const GAMEMODE_IFACE: &str = "com.feralinteractive.GameMode";

/// Lo que se espera a que salir termine al recibir una señal de fin.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(3);

/// Una acción del modo juego.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    DoNotDisturb,
    DisableAnimations,
    Gamemode,
}

/// Una entrada del registro: la acción, su clave en `game_mode` y si va
/// encendida cuando la configuración no dice nada.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub action: Action,
    pub key: &'static str,
    pub default: bool,
}

/// El registro, en el orden en que se aplican. Al salir se deshacen al revés.
pub const REGISTRY: [Entry; 3] = [
    Entry {
        action: Action::DoNotDisturb,
        key: "do_not_disturb",
        default: true,
    },
    Entry {
        action: Action::DisableAnimations,
        key: "disable_animations",
        default: true,
    },
    Entry {
        action: Action::Gamemode,
        key: "gamemode",
        default: true,
    },
];

/// Las acciones encendidas según el contenido de `vasak.conf`.
///
/// Una clave que falta —o que no es un booleano— toma su valor por omisión, y
/// una que el registro no conoce se ignora: así una versión vieja del
/// escritorio no se rompe con una configuración nueva, ni al revés. Un archivo
/// que no se puede leer vale lo mismo que uno vacío.
pub fn actions_from_config(content: &str) -> Vec<Action> {
    let parsed: serde_json::Value = serde_json::from_str(content).unwrap_or_default();
    let section = parsed
        .get(CONFIG_SECTION)
        .and_then(serde_json::Value::as_object);
    REGISTRY
        .iter()
        .filter(|entry| {
            section
                .and_then(|section| section.get(entry.key))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(entry.default)
        })
        .map(|entry| entry.action)
        .collect()
}

/// Lee `vasak.conf` una vez, el mismo archivo que maneja
/// `tauri-plugin-config-manager` (que desde la 2.6 conserva las secciones que
/// no conoce, como ésta).
fn read_actions() -> Vec<Action> {
    let content = crate::panel_position::config_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default();
    actions_from_config(&content)
}

/// Lo necesario para deshacer una acción aplicada. Es lo que va al archivo de
/// recuperación.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Undo {
    /// «No molestar» estaba quitado y se puso. `revision` es la de
    /// [`crate::do_not_disturb::revision`] justo después de ponerlo.
    DoNotDisturb { revision: u64 },
    /// El valor que tenía cada opción que se pasó a `none`.
    DisableAnimations { previous: BTreeMap<String, String> },
    /// El pid que se registró en `gamemoded`.
    Gamemode { pid: i32 },
}

/// Lo que el modo juego toca del sistema. Aparte para poder probar la lógica
/// sin el demonio de notificaciones, sin Wayfire y sin `gamemoded`.
#[async_trait]
pub trait Host: Send + Sync {
    /// Pone «No molestar» y devuelve el anterior.
    async fn set_do_not_disturb(&self, enabled: bool) -> Result<bool, String>;
    /// Ver [`crate::do_not_disturb::revision`].
    fn do_not_disturb_revision(&self) -> u64;
    /// El valor actual de una opción de Wayfire.
    async fn wayfire_option(&self, option: &str) -> Result<String, String>;
    /// Cambia opciones de Wayfire en memoria, todas en una llamada.
    async fn set_wayfire_options(&self, options: &BTreeMap<String, String>) -> Result<(), String>;
    /// Si `gamemoded` está instalado (activable) o corriendo.
    async fn gamemode_installed(&self) -> bool;
    async fn register_gamemode(&self, pid: i32) -> Result<(), String>;
    async fn unregister_gamemode(&self, pid: i32) -> Result<(), String>;
    /// El pid de este proceso.
    fn pid(&self) -> i32;
}

impl Action {
    /// Aplica la acción. `None` si no había nada que cambiar —«No molestar» ya
    /// puesto, animaciones ya apagadas, sin `gamemoded`—, y entonces tampoco hay
    /// nada que deshacer.
    pub async fn enter(self, host: &dyn Host) -> Result<Option<Undo>, String> {
        match self {
            Action::DoNotDisturb => {
                let previous = host.set_do_not_disturb(true).await?;
                if previous {
                    return Ok(None);
                }
                Ok(Some(Undo::DoNotDisturb {
                    revision: host.do_not_disturb_revision(),
                }))
            }
            Action::DisableAnimations => {
                let mut previous = BTreeMap::new();
                for option in ANIMATION_OPTIONS {
                    let value = host.wayfire_option(option).await?;
                    if value != NO_ANIMATION {
                        previous.insert(option.to_string(), value);
                    }
                }
                if previous.is_empty() {
                    return Ok(None);
                }
                let off = previous
                    .keys()
                    .map(|option| (option.clone(), NO_ANIMATION.to_string()))
                    .collect();
                host.set_wayfire_options(&off).await?;
                Ok(Some(Undo::DisableAnimations { previous }))
            }
            Action::Gamemode => {
                if !host.gamemode_installed().await {
                    log_info("[game-mode] gamemoded no está instalado: se saltea");
                    return Ok(None);
                }
                let pid = host.pid();
                host.register_gamemode(pid).await?;
                Ok(Some(Undo::Gamemode { pid }))
            }
        }
    }
}

impl Undo {
    /// Deshace la acción al salir del modo, en la misma sesión.
    pub async fn exit(&self, host: &dyn Host) -> Result<(), String> {
        match self {
            Undo::DoNotDisturb { revision } => {
                if host.do_not_disturb_revision() != *revision {
                    log_info(
                        "[game-mode] «No molestar» se tocó durante el juego: se deja como está",
                    );
                    return Ok(());
                }
                host.set_do_not_disturb(false).await.map(|_| ())
            }
            Undo::DisableAnimations { .. } => self.restore_animations(host).await,
            Undo::Gamemode { pid } => host.unregister_gamemode(*pid).await,
        }
    }

    /// Deshace la acción después de una caída del escritorio.
    pub async fn restore(&self, host: &dyn Host) -> Result<(), String> {
        match self {
            // La revisión era de otro proceso: no dice nada en éste. Se quita.
            Undo::DoNotDisturb { .. } => host.set_do_not_disturb(false).await.map(|_| ()),
            Undo::DisableAnimations { .. } => self.restore_animations(host).await,
            // El proceso registrado murió y el recolector de gamemoded ya lo dio
            // de baja. Si por casualidad es este mismo pid, se lo da de baja acá.
            Undo::Gamemode { pid } => {
                if *pid == host.pid() {
                    host.unregister_gamemode(*pid).await
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Devuelve cada opción a su valor, salvo las que ya no están en `none`.
    async fn restore_animations(&self, host: &dyn Host) -> Result<(), String> {
        let Undo::DisableAnimations { previous } = self else {
            return Ok(());
        };
        let mut back = BTreeMap::new();
        for (option, value) in previous {
            if host.wayfire_option(option).await? == NO_ANIMATION {
                back.insert(option.clone(), value.clone());
            }
        }
        if back.is_empty() {
            return Ok(());
        }
        host.set_wayfire_options(&back).await
    }
}

/// El archivo de recuperación, si hay `$XDG_RUNTIME_DIR`.
pub fn recovery_path() -> Option<PathBuf> {
    dirs::runtime_dir().map(|dir| dir.join(RECOVERY_FILE))
}

/// Escribe lo aplicado. Por un temporal y `rename`, así una caída en el medio
/// no deja un archivo a medias.
pub fn write_recovery(path: &Path, undo: &[Undo]) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(undo)?)?;
    std::fs::rename(tmp, path)
}

/// Lee lo que quedó. Un archivo ilegible no se puede restaurar: vale vacío.
pub fn read_recovery(path: &Path) -> Option<Vec<Undo>> {
    let content = std::fs::read(path).ok()?;
    match serde_json::from_slice(&content) {
        Ok(undo) => Some(undo),
        Err(e) => {
            log_warning(&format!(
                "[game-mode] archivo de recuperación ilegible: {e}"
            ));
            Some(Vec::new())
        }
    }
}

fn remove_recovery(path: Option<&Path>) {
    if let Some(path) = path {
        if let Err(e) = std::fs::remove_file(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log_error(&format!(
                    "[game-mode] no se pudo borrar {}: {e}",
                    path.display()
                ));
            }
        }
    }
}

/// Lo que el candado protege.
#[derive(Default)]
struct State {
    /// `Some` mientras el modo está puesto, con lo que hay que deshacer.
    session: Option<Vec<Undo>>,
    /// Lo que no se pudo deshacer la última vez —Wayfire no contestó, el bus
    /// se cortó—. Sigue en el archivo de recuperación y se vuelve a intentar
    /// al salir la próxima vez o al arrancar el próximo escritorio.
    leftover: Vec<Undo>,
}

/// El estado del modo. Aparte de la global para que cada prueba tenga el suyo.
pub struct Controller {
    state: Mutex<State>,
    /// La copia para preguntar sin esperar el candado.
    active: AtomicBool,
    recovery: Option<PathBuf>,
}

impl Controller {
    pub fn new(recovery: Option<PathBuf>) -> Self {
        Self {
            state: Mutex::new(State::default()),
            active: AtomicBool::new(false),
            recovery,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    /// Pone o saca el modo y devuelve el estado anterior. `actions` se llama
    /// sólo al entrar: es la única lectura de la configuración.
    ///
    /// Una acción que falla se anota en el registro y no frena a las demás: el
    /// modo entra igual con lo que se pudo, y sale deshaciendo todo lo que
    /// pueda.
    pub async fn set(
        &self,
        host: &dyn Host,
        enabled: bool,
        actions: impl FnOnce() -> Vec<Action>,
    ) -> bool {
        let mut state = self.state.lock().await;
        let previous = state.session.is_some();
        if previous == enabled {
            return previous;
        }
        if enabled {
            // Lo que quedó sin deshacer va primero: se deshace último, después
            // de lo de esta vez, y se reintenta al salir.
            let mut undo = std::mem::take(&mut state.leftover);
            for action in actions() {
                match action.enter(host).await {
                    Ok(Some(step)) => {
                        undo.push(step);
                        self.save(&undo);
                    }
                    Ok(None) => {}
                    Err(e) => log_error(&format!("[game-mode] {action:?} no se aplicó: {e}")),
                }
            }
            state.session = Some(undo);
        } else if let Some(undo) = state.session.take() {
            let mut failed = Vec::new();
            for step in undo.into_iter().rev() {
                if let Err(e) = step.exit(host).await {
                    log_error(&format!("[game-mode] no se pudo deshacer {step:?}: {e}"));
                    failed.push(step);
                }
            }
            failed.reverse();
            self.keep(&failed);
            state.leftover = failed;
        }
        self.active.store(enabled, Ordering::Release);
        previous
    }

    /// Restaura lo que dejó un escritorio que se cayó con el modo puesto.
    /// Devuelve si había algo.
    pub async fn recover(&self, host: &dyn Host) -> bool {
        let Some(path) = self.recovery.as_deref() else {
            return false;
        };
        let mut state = self.state.lock().await;
        let Some(undo) = read_recovery(path) else {
            return false;
        };
        log_info("[game-mode] el escritorio se cayó con el modo puesto: restaurando");
        let mut failed = Vec::new();
        for step in undo.into_iter().rev() {
            if let Err(e) = step.restore(host).await {
                log_error(&format!("[game-mode] no se pudo restaurar {step:?}: {e}"));
                failed.push(step);
            }
        }
        failed.reverse();
        self.keep(&failed);
        state.leftover = failed;
        true
    }

    /// Deja en el archivo sólo lo que falta deshacer, o lo borra si no falta
    /// nada.
    fn keep(&self, failed: &[Undo]) {
        if failed.is_empty() {
            remove_recovery(self.recovery.as_deref());
        } else {
            self.save(failed);
        }
    }

    fn save(&self, undo: &[Undo]) {
        if let Some(path) = self.recovery.as_deref() {
            if let Err(e) = write_recovery(path, undo) {
                log_error(&format!(
                    "[game-mode] no se pudo anotar la recuperación: {e}"
                ));
            }
        }
    }
}

/// Lo de verdad: el demonio de notificaciones, Wayfire y el bus de sesión.
pub struct SystemHost;

async fn session_bus() -> Result<zbus::Connection, String> {
    crate::notifications::connection().await
}

/// Llama a `RegisterGame` o `UnregisterGame`. `gamemoded` contesta un entero:
/// negativo es que no aceptó.
pub async fn gamemode_call(conn: &zbus::Connection, method: &str, pid: i32) -> Result<(), String> {
    let proxy = zbus::Proxy::new(conn, GAMEMODE_DEST, GAMEMODE_PATH, GAMEMODE_IFACE)
        .await
        .map_err(|e| e.to_string())?;
    let status: i32 = proxy
        .call(method, &(pid,))
        .await
        .map_err(|e| format!("gamemoded no aceptó {method}: {e}"))?;
    if status < 0 {
        return Err(format!("gamemoded rechazó {method} ({status})"));
    }
    Ok(())
}

/// Si el nombre de `gamemoded` está en el bus o se puede activar.
pub async fn gamemode_on_bus(conn: &zbus::Connection) -> bool {
    let Ok(dbus) = zbus::fdo::DBusProxy::new(conn).await else {
        return false;
    };
    let Ok(name) = zbus::names::BusName::try_from(GAMEMODE_DEST) else {
        return false;
    };
    if dbus.name_has_owner(name).await.unwrap_or(false) {
        return true;
    }
    dbus.list_activatable_names()
        .await
        .map(|names| names.iter().any(|name| name.as_str() == GAMEMODE_DEST))
        .unwrap_or(false)
}

#[async_trait]
impl Host for SystemHost {
    async fn set_do_not_disturb(&self, enabled: bool) -> Result<bool, String> {
        crate::do_not_disturb::set_enabled(enabled).await
    }

    fn do_not_disturb_revision(&self) -> u64 {
        crate::do_not_disturb::revision()
    }

    async fn wayfire_option(&self, option: &str) -> Result<String, String> {
        let client = crate::window_manager::wayfire_ipc::get_wayfire_client()
            .await
            .ok_or("Wayfire no contesta")?;
        let response = client
            .get_config_option(option)
            .await
            .map_err(|e| e.to_string())?;
        response
            .get("value")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("Wayfire no dio el valor de {option}"))
    }

    async fn set_wayfire_options(&self, options: &BTreeMap<String, String>) -> Result<(), String> {
        let client = crate::window_manager::wayfire_ipc::get_wayfire_client()
            .await
            .ok_or("Wayfire no contesta")?;
        client
            .set_config_options(options)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    async fn gamemode_installed(&self) -> bool {
        match session_bus().await {
            Ok(conn) => gamemode_on_bus(&conn).await,
            Err(_) => false,
        }
    }

    async fn register_gamemode(&self, pid: i32) -> Result<(), String> {
        gamemode_call(&session_bus().await?, "RegisterGame", pid).await
    }

    async fn unregister_gamemode(&self, pid: i32) -> Result<(), String> {
        gamemode_call(&session_bus().await?, "UnregisterGame", pid).await
    }

    fn pid(&self) -> i32 {
        i32::try_from(std::process::id()).unwrap_or(i32::MAX)
    }
}

static CONTROLLER: OnceLock<Controller> = OnceLock::new();
static APP: OnceLock<AppHandle> = OnceLock::new();
static SHUTDOWN_HOOK: OnceLock<()> = OnceLock::new();

fn controller() -> &'static Controller {
    CONTROLLER.get_or_init(|| Controller::new(recovery_path()))
}

/// Si el modo está puesto, sin esperar nada.
pub fn is_active() -> bool {
    controller().is_active()
}

/// Pone o saca el modo, avisa a las ventanas si cambió y devuelve el anterior.
pub async fn set_enabled(enabled: bool) -> bool {
    if enabled {
        install_shutdown_hook();
    }
    let previous = controller().set(&SystemHost, enabled, read_actions).await;
    if previous != enabled {
        log_info(&format!(
            "[game-mode] {}",
            if enabled { "puesto" } else { "quitado" }
        ));
        if let Some(app) = APP.get() {
            let _ = app.emit(CHANGED_EVENT, enabled);
        }
    }
    previous
}

/// Al recibir `SIGTERM` o `SIGINT` con el modo puesto, sale de él antes de
/// terminar: «No molestar» lo guarda el servidor de notificaciones entre
/// sesiones, y una sesión que se cierra con el modo puesto no lo puede dejar
/// prendido para la próxima.
///
/// Se instala recién la primera vez que se pone el modo, así un escritorio que
/// nunca lo usa termina como siempre. Instalado, la señal ya no mata el proceso
/// sola: se termina a mano con 0, que para systemd es un final limpio igual que
/// el `SIGTERM` de antes.
fn install_shutdown_hook() {
    if SHUTDOWN_HOOK.set(()).is_err() {
        return;
    }
    tauri::async_runtime::spawn(async {
        use tokio::signal::unix::{signal, SignalKind};
        let (Ok(mut term), Ok(mut int)) = (
            signal(SignalKind::terminate()),
            signal(SignalKind::interrupt()),
        ) else {
            log_error("[game-mode] no se pudo escuchar el fin del proceso");
            return;
        };
        tokio::select! {
            _ = term.recv() => {}
            _ = int.recv() => {}
        }
        if is_active() {
            log_info("[game-mode] el escritorio termina con el modo puesto: saliendo de él");
            let _ = tokio::time::timeout(SHUTDOWN_GRACE, set_enabled(false)).await;
        }
        crate::logger::flush();
        std::process::exit(0);
    });
}

/// Al arrancar: guarda el `AppHandle` para los avisos y restaura lo que haya
/// dejado un escritorio caído.
pub fn start(app: AppHandle) {
    let _ = APP.set(app);
    tauri::async_runtime::spawn(async {
        controller().recover(&SystemHost).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tray::bus_tests::PrivateBus;
    use std::sync::atomic::AtomicU64;
    use std::sync::Mutex as StdMutex;

    /// Un sistema de mentira: «No molestar», las opciones de Wayfire y
    /// `gamemoded`, todo en memoria, con lo que se le pidió anotado.
    struct FakeHost {
        dnd: AtomicBool,
        dnd_available: bool,
        revision: AtomicU64,
        wayfire: StdMutex<BTreeMap<String, String>>,
        wayfire_calls: AtomicU64,
        wayfire_down: AtomicBool,
        gamemode: Option<StdMutex<Vec<i32>>>,
        pid: i32,
    }

    impl FakeHost {
        fn new() -> Self {
            let wayfire = [
                ("animate/open_animation", "fade"),
                ("animate/close_animation", "zoom"),
                ("animate/minimize_animation", "squeezimize"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
            Self {
                dnd: AtomicBool::new(false),
                dnd_available: true,
                revision: AtomicU64::new(0),
                wayfire: StdMutex::new(wayfire),
                wayfire_calls: AtomicU64::new(0),
                wayfire_down: AtomicBool::new(false),
                gamemode: Some(StdMutex::new(Vec::new())),
                pid: 4242,
            }
        }

        fn without_gamemode(mut self) -> Self {
            self.gamemode = None;
            self
        }

        /// Alguien más toca «No molestar», como el mosaico o la bandeja.
        fn touch_dnd(&self, enabled: bool) {
            if self.dnd.swap(enabled, Ordering::SeqCst) != enabled {
                self.revision.fetch_add(1, Ordering::SeqCst);
            }
        }

        fn option(&self, option: &str) -> String {
            self.wayfire.lock().unwrap()[option].clone()
        }

        fn registered(&self) -> Vec<i32> {
            self.gamemode.as_ref().unwrap().lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl Host for FakeHost {
        async fn set_do_not_disturb(&self, enabled: bool) -> Result<bool, String> {
            if !self.dnd_available {
                return Err("sin demonio".into());
            }
            let previous = self.dnd.load(Ordering::SeqCst);
            self.touch_dnd(enabled);
            Ok(previous)
        }

        fn do_not_disturb_revision(&self) -> u64 {
            self.revision.load(Ordering::SeqCst)
        }

        async fn wayfire_option(&self, option: &str) -> Result<String, String> {
            if self.wayfire_down.load(Ordering::SeqCst) {
                return Err("Wayfire no contesta".into());
            }
            self.wayfire
                .lock()
                .unwrap()
                .get(option)
                .cloned()
                .ok_or_else(|| format!("{option}: Option not found!"))
        }

        async fn set_wayfire_options(
            &self,
            options: &BTreeMap<String, String>,
        ) -> Result<(), String> {
            self.wayfire_calls.fetch_add(1, Ordering::SeqCst);
            let mut wayfire = self.wayfire.lock().unwrap();
            for (k, v) in options {
                wayfire.insert(k.clone(), v.clone());
            }
            Ok(())
        }

        async fn gamemode_installed(&self) -> bool {
            self.gamemode.is_some()
        }

        async fn register_gamemode(&self, pid: i32) -> Result<(), String> {
            let Some(list) = &self.gamemode else {
                return Err("no está".into());
            };
            list.lock().unwrap().push(pid);
            Ok(())
        }

        async fn unregister_gamemode(&self, pid: i32) -> Result<(), String> {
            let Some(list) = &self.gamemode else {
                return Err("no está".into());
            };
            list.lock().unwrap().retain(|p| *p != pid);
            Ok(())
        }

        fn pid(&self) -> i32 {
            self.pid
        }
    }

    fn all() -> Vec<Action> {
        REGISTRY.iter().map(|entry| entry.action).collect()
    }

    fn temp_recovery(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("vasak-game-mode-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(RECOVERY_FILE);
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn sin_configuracion_van_las_tres() {
        assert_eq!(actions_from_config(""), all());
        assert_eq!(actions_from_config("{}"), all());
        assert_eq!(actions_from_config("no es json"), all());
    }

    #[test]
    fn la_clave_que_falta_toma_el_valor_por_omision() {
        let actions = actions_from_config(r#"{"game_mode":{"gamemode":false}}"#);
        assert_eq!(
            actions,
            vec![Action::DoNotDisturb, Action::DisableAnimations]
        );
    }

    #[test]
    fn la_clave_desconocida_se_ignora() {
        let actions = actions_from_config(
            r#"{"game_mode":{"do_not_disturb":false,"night_light":true,"disable_animations":"sí"}}"#,
        );
        // `night_light` no existe todavía y un valor que no es booleano vale
        // por omisión.
        assert_eq!(actions, vec![Action::DisableAnimations, Action::Gamemode]);
    }

    #[test]
    fn todas_apagadas_no_aplica_nada() {
        let actions = actions_from_config(
            r#"{"panel":{"position":"top"},"game_mode":{"do_not_disturb":false,"disable_animations":false,"gamemode":false}}"#,
        );
        assert!(actions.is_empty());
    }

    #[test]
    fn lo_aplicado_se_serializa_con_la_accion() {
        let undo = vec![
            Undo::DoNotDisturb { revision: 3 },
            Undo::Gamemode { pid: 10 },
        ];
        let json = serde_json::to_value(&undo).unwrap();
        assert_eq!(
            json,
            serde_json::json!([
                { "action": "do_not_disturb", "revision": 3 },
                { "action": "gamemode", "pid": 10 }
            ])
        );
        assert_eq!(serde_json::from_value::<Vec<Undo>>(json).unwrap(), undo);
    }

    #[tokio::test]
    async fn entrar_y_salir_deja_todo_como_estaba() {
        let host = FakeHost::new();
        let controller = Controller::new(None);

        assert!(!controller.set(&host, true, all).await);
        assert!(controller.is_active());
        assert!(host.dnd.load(Ordering::SeqCst));
        for option in ANIMATION_OPTIONS {
            assert_eq!(host.option(option), NO_ANIMATION);
        }
        assert_eq!(
            host.wayfire_calls.load(Ordering::SeqCst),
            1,
            "una sola llamada"
        );
        assert_eq!(host.registered(), vec![4242]);

        assert!(controller.set(&host, false, Vec::new).await);
        assert!(!controller.is_active());
        assert!(!host.dnd.load(Ordering::SeqCst));
        assert_eq!(host.option("animate/open_animation"), "fade");
        assert_eq!(host.option("animate/close_animation"), "zoom");
        assert_eq!(host.option("animate/minimize_animation"), "squeezimize");
        assert!(host.registered().is_empty());
    }

    #[tokio::test]
    async fn la_configuracion_se_lee_solo_al_entrar() {
        let host = FakeHost::new();
        let controller = Controller::new(None);
        let reads = AtomicU64::new(0);
        let read = || {
            reads.fetch_add(1, Ordering::SeqCst);
            all()
        };
        controller.set(&host, true, read).await;
        controller
            .set(&host, true, || panic!("ya estaba puesto"))
            .await;
        controller
            .set(&host, false, || panic!("salir no lee"))
            .await;
        assert_eq!(reads.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn solo_aplica_las_acciones_encendidas() {
        let host = FakeHost::new();
        let controller = Controller::new(None);
        controller
            .set(&host, true, || {
                actions_from_config(r#"{"game_mode":{"disable_animations":false}}"#)
            })
            .await;
        assert!(host.dnd.load(Ordering::SeqCst));
        assert_eq!(host.option("animate/open_animation"), "fade");
        assert_eq!(host.wayfire_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn no_molestar_tocado_durante_el_juego_no_se_pisa() {
        let host = FakeHost::new();
        let controller = Controller::new(None);
        controller.set(&host, true, all).await;

        // Durante el juego alguien lo quita y lo vuelve a poner desde el mosaico.
        host.touch_dnd(false);
        host.touch_dnd(true);

        controller.set(&host, false, Vec::new).await;
        assert!(
            host.dnd.load(Ordering::SeqCst),
            "lo dejó puesto alguien más: se respeta"
        );
    }

    #[tokio::test]
    async fn no_molestar_ya_puesto_se_queda_puesto() {
        let host = FakeHost::new();
        host.touch_dnd(true);
        let controller = Controller::new(None);
        controller.set(&host, true, all).await;
        controller.set(&host, false, Vec::new).await;
        assert!(host.dnd.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn la_animacion_cambiada_durante_el_juego_no_se_pisa() {
        let host = FakeHost::new();
        let controller = Controller::new(None);
        controller.set(&host, true, all).await;
        // Configuración reescribió wayfire.ini y Wayfire lo releyó.
        host.wayfire
            .lock()
            .unwrap()
            .insert("animate/open_animation".into(), "spin".into());
        controller.set(&host, false, Vec::new).await;
        assert_eq!(host.option("animate/open_animation"), "spin");
        assert_eq!(host.option("animate/close_animation"), "zoom");
    }

    #[tokio::test]
    async fn sin_gamemoded_no_falla() {
        let host = FakeHost::new().without_gamemode();
        let controller = Controller::new(None);
        assert!(!controller.set(&host, true, all).await);
        assert!(controller.is_active(), "el modo entra igual");
        assert!(host.dnd.load(Ordering::SeqCst));
        assert_eq!(host.option("animate/open_animation"), NO_ANIMATION);
        assert!(controller.set(&host, false, Vec::new).await);
        assert_eq!(host.option("animate/open_animation"), "fade");
    }

    #[tokio::test]
    async fn una_accion_que_falla_no_frena_a_las_demas() {
        let mut host = FakeHost::new();
        host.dnd_available = false;
        let controller = Controller::new(None);
        controller.set(&host, true, all).await;
        assert!(controller.is_active());
        assert_eq!(host.option("animate/open_animation"), NO_ANIMATION);
        assert_eq!(host.registered(), vec![4242]);
        controller.set(&host, false, Vec::new).await;
        assert_eq!(host.option("animate/open_animation"), "fade");
        assert!(host.registered().is_empty());
    }

    #[tokio::test]
    async fn se_anota_mientras_dura_y_se_borra_al_salir() {
        let path = temp_recovery("anota");
        let host = FakeHost::new();
        let controller = Controller::new(Some(path.clone()));
        controller.set(&host, true, all).await;
        let saved = read_recovery(&path).expect("quedó anotado");
        assert_eq!(saved.len(), 3);
        assert_eq!(saved[0], Undo::DoNotDisturb { revision: 1 });
        assert_eq!(saved[2], Undo::Gamemode { pid: 4242 });
        controller.set(&host, false, Vec::new).await;
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn tras_una_caida_se_restaura_al_volver() {
        let path = temp_recovery("caida");
        // El escritorio que se cae.
        let before = FakeHost::new();
        Controller::new(Some(path.clone()))
            .set(&before, true, all)
            .await;
        let saved = std::fs::read(&path).unwrap();

        // El que arranca después: otro pid, y el compositor y el demonio de
        // notificaciones como los dejó el anterior.
        let mut after = FakeHost::new();
        after.pid = 5151;
        after.touch_dnd(true);
        *after.wayfire.lock().unwrap() = before.wayfire.lock().unwrap().clone();
        std::fs::write(&path, saved).unwrap();

        let controller = Controller::new(Some(path.clone()));
        assert!(controller.recover(&after).await);
        assert!(!after.dnd.load(Ordering::SeqCst));
        assert_eq!(after.option("animate/open_animation"), "fade");
        assert_eq!(after.option("animate/minimize_animation"), "squeezimize");
        assert!(after.registered().is_empty(), "el pid muerto no se toca");
        assert!(!controller.is_active(), "no se recuerda");
        assert!(!path.exists());
        assert!(!controller.recover(&after).await, "una sola vez");
    }

    /// Si Wayfire no contesta al salir, lo que no se deshizo queda en el
    /// archivo y se reintenta la próxima vez que se sale.
    #[tokio::test]
    async fn lo_que_no_se_pudo_deshacer_queda_anotado() {
        let path = temp_recovery("pendiente");
        let host = FakeHost::new();
        let controller = Controller::new(Some(path.clone()));
        controller.set(&host, true, all).await;

        host.wayfire_down.store(true, Ordering::SeqCst);
        controller.set(&host, false, Vec::new).await;
        assert!(!controller.is_active());
        assert!(!host.dnd.load(Ordering::SeqCst), "lo demás se deshizo");
        assert!(host.registered().is_empty());
        let pending = read_recovery(&path).expect("queda anotado");
        assert!(matches!(
            pending.as_slice(),
            [Undo::DisableAnimations { .. }]
        ));

        // Wayfire vuelve; la próxima vuelta lo deshace también.
        host.wayfire_down.store(false, Ordering::SeqCst);
        controller.set(&host, true, all).await;
        controller.set(&host, false, Vec::new).await;
        assert_eq!(host.option("animate/open_animation"), "fade");
        assert_eq!(host.option("animate/minimize_animation"), "squeezimize");
        assert!(!path.exists());
    }

    /// Lo mismo al restaurar tras una caída: si Wayfire todavía no contesta,
    /// el archivo no se borra.
    #[tokio::test]
    async fn la_recuperacion_que_falla_no_borra_el_archivo() {
        let path = temp_recovery("recupera-falla");
        let before = FakeHost::new();
        Controller::new(Some(path.clone()))
            .set(&before, true, all)
            .await;

        let after = FakeHost::new();
        *after.wayfire.lock().unwrap() = before.wayfire.lock().unwrap().clone();
        after.wayfire_down.store(true, Ordering::SeqCst);
        assert!(Controller::new(Some(path.clone())).recover(&after).await);
        assert!(path.exists());

        after.wayfire_down.store(false, Ordering::SeqCst);
        assert!(Controller::new(Some(path.clone())).recover(&after).await);
        assert_eq!(after.option("animate/open_animation"), "fade");
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn un_archivo_ilegible_se_descarta() {
        let path = temp_recovery("ilegible");
        std::fs::write(&path, b"{roto").unwrap();
        let host = FakeHost::new();
        assert!(Controller::new(Some(path.clone())).recover(&host).await);
        assert!(!path.exists());
        assert_eq!(host.option("animate/open_animation"), "fade");
    }

    /// `gamemoded` de mentira con la interfaz de la 1.8.
    struct FakeGameMode {
        registered: std::sync::Arc<StdMutex<Vec<i32>>>,
    }

    #[zbus::interface(name = "com.feralinteractive.GameMode")]
    impl FakeGameMode {
        async fn register_game(&self, pid: i32) -> i32 {
            self.registered.lock().unwrap().push(pid);
            0
        }
        async fn unregister_game(&self, pid: i32) -> i32 {
            let mut list = self.registered.lock().unwrap();
            let before = list.len();
            list.retain(|p| *p != pid);
            if list.len() == before {
                -1
            } else {
                0
            }
        }
    }

    #[tokio::test]
    async fn gamemoded_se_detecta_en_el_bus() {
        let bus = crate::tray::bus_tests::private_bus!();
        let client = bus.connect().await;
        assert!(!gamemode_on_bus(&client).await, "sin gamemoded");

        let registered = std::sync::Arc::new(StdMutex::new(Vec::new()));
        let _daemon = bus
            .connect_serving(|builder| {
                builder
                    .serve_at(
                        GAMEMODE_PATH,
                        FakeGameMode {
                            registered: registered.clone(),
                        },
                    )?
                    .name(GAMEMODE_DEST)
            })
            .await;
        assert!(gamemode_on_bus(&client).await);

        gamemode_call(&client, "RegisterGame", 77).await.unwrap();
        assert_eq!(*registered.lock().unwrap(), vec![77]);
        gamemode_call(&client, "UnregisterGame", 77).await.unwrap();
        assert!(registered.lock().unwrap().is_empty());
        assert!(
            gamemode_call(&client, "UnregisterGame", 77).await.is_err(),
            "un -1 de gamemoded es un error"
        );
    }
}
