//! «Modo avión» (vasak-desktop#180): apaga todas las radios y, al quitarlo,
//! vuelve a encender sólo las que estaban encendidas.
//!
//! Todo pasa por `/dev/rfkill`, el dispositivo del kernel que bloquea y
//! desbloquea radios. logind le da al usuario de la sesión activa acceso de
//! escritura con una ACL (`user:<usuario>:rw-`), así que no hace falta `sudo`
//! ni un subproceso (`rfkill block all`).
//!
//! - **Sin sondeos.** Al abrir `/dev/rfkill` el kernel encola un evento `ADD`
//!   por cada radio, y después uno por cada cambio, venga de donde venga: este
//!   proceso, NetworkManager, BlueZ o la tecla de avión del teclado. El fd se
//!   lee desde el runtime asíncrono ([`AsyncFd`]): no hay hilo propio, y la
//!   tarea sólo se despierta cuando el kernel tiene algo para contar.
//! - **Una copia en memoria** ([`Mirror`]) con cada radio y su bloqueo.
//!   Preguntar el estado ([`state`]) no toca el dispositivo.
//! - **Qué restaurar.** Al ponerlo se anota qué radios estaban encendidas; al
//!   quitarlo se desbloquean sólo ésas. Si el modo lo puso otro (la tecla, otra
//!   herramienta) no hay nota, y se desbloquean todas, como hace GNOME.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::unix::AsyncFd;

use crate::logger::{log_error, log_info};

const RFKILL_PATH: &str = "/dev/rfkill";

/// El evento que reciben las ventanas con cada cambio, con un
/// [`AirplaneModeState`] adentro.
pub const CHANGED_EVENT: &str = "airplane-mode-changed";

/// `struct rfkill_event` de `<linux/rfkill.h>`: `idx` (u32, orden del
/// equipo), `type`, `op`, `soft` y `hard` (u8). Ocho bytes. Los kernels desde
/// el 5.11 pueden agregar `hard_block_reasons` (`rfkill_event_ext`, nueve),
/// pero sólo a quien lo pida con `RFKILL_IOCTL_MAX_SIZE`; igual se acepta.
pub const EVENT_SIZE: usize = 8;

/// `RFKILL_TYPE_ALL`: en un `CHANGE_ALL`, todos los tipos.
pub const TYPE_ALL: u8 = 0;
pub const TYPE_WLAN: u8 = 1;
pub const TYPE_BLUETOOTH: u8 = 2;
/// `NUM_RFKILL_TYPES`: de `WLAN` (1) a `NFC` (8). Un tipo más nuevo que el
/// kernel invente también se bloquea con `TYPE_ALL`.
const NUM_TYPES: u8 = 9;

/// `enum rfkill_operation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Apareció una radio (y, al abrir el dispositivo, una por cada una).
    Add = 0,
    /// Se fue una radio.
    Del = 1,
    /// Cambió el bloqueo de una radio; escrito, cambia el de `idx`.
    Change = 2,
    /// Escrito, cambia el de todas las radios de un tipo (o de todos, con
    /// [`TYPE_ALL`]) y el estado con que nacen las que se conecten después.
    ChangeAll = 3,
}

impl Op {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Add),
            1 => Some(Self::Del),
            2 => Some(Self::Change),
            3 => Some(Self::ChangeAll),
            _ => None,
        }
    }
}

/// Un `struct rfkill_event`, leído o por escribir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RfkillEvent {
    pub idx: u32,
    pub kind: u8,
    pub op: Op,
    /// Bloqueada por software: lo que se puede cambiar desde acá.
    pub soft: bool,
    /// Bloqueada por hardware: un interruptor, o el firmware. No se puede
    /// cambiar desde acá.
    pub hard: bool,
}

impl RfkillEvent {
    /// Lee un evento tal como lo entrega `read(2)`. Con menos de ocho bytes, o
    /// con una operación que no se conoce, no hay evento.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < EVENT_SIZE {
            return None;
        }
        Some(Self {
            idx: u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            kind: bytes[4],
            op: Op::from_u8(bytes[5])?,
            soft: bytes[6] != 0,
            hard: bytes[7] != 0,
        })
    }

    /// Los ocho bytes que espera `write(2)`.
    pub fn encode(&self) -> [u8; EVENT_SIZE] {
        let idx = self.idx.to_ne_bytes();
        [
            idx[0],
            idx[1],
            idx[2],
            idx[3],
            self.kind,
            self.op as u8,
            u8::from(self.soft),
            u8::from(self.hard),
        ]
    }

    /// Bloquear o desbloquear todas las radios de `kind`.
    pub fn change_all(kind: u8, block: bool) -> Self {
        Self {
            idx: 0,
            kind,
            op: Op::ChangeAll,
            soft: block,
            hard: false,
        }
    }

    /// Bloquear o desbloquear una radio.
    pub fn change(idx: u32, block: bool) -> Self {
        Self {
            idx,
            kind: TYPE_ALL,
            op: Op::Change,
            soft: block,
            hard: false,
        }
    }
}

/// Una radio, como la cuenta el kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Radio {
    pub kind: u8,
    pub soft: bool,
    pub hard: bool,
}

impl Radio {
    fn blocked(&self) -> bool {
        self.soft || self.hard
    }
}

/// Las radios del equipo, por `idx`.
pub type Radios = BTreeMap<u32, Radio>;

/// Lo que se sabe del modo, para el frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AirplaneModeState {
    /// Hay radios y se pueden bloquear. Sin radios, o sin permiso sobre
    /// `/dev/rfkill`, el mosaico se ve **no disponible**.
    pub available: bool,
    /// Hay radios y están todas bloqueadas.
    pub enabled: bool,
    /// Alguna está bloqueada por hardware: quitar el modo no la va a prender.
    pub hardware: bool,
    /// Hay Wi-Fi y está todo bloqueado: el mosaico de red se ve apagado
    /// aunque NetworkManager todavía no se haya enterado.
    pub wlan_blocked: bool,
    /// Lo mismo para Bluetooth.
    pub bluetooth_blocked: bool,
}

/// Si hay radios de `kind` y están todas bloqueadas.
fn all_blocked(radios: &Radios, kind: u8) -> bool {
    let mut of_kind = radios.values().filter(|r| r.kind == kind).peekable();
    of_kind.peek().is_some() && of_kind.all(Radio::blocked)
}

/// El estado que se deriva de las radios.
pub fn derive(radios: &Radios, writable: bool) -> AirplaneModeState {
    let any = !radios.is_empty();
    AirplaneModeState {
        available: any && writable,
        enabled: any && radios.values().all(Radio::blocked),
        hardware: radios.values().any(|r| r.hard),
        wlan_blocked: all_blocked(radios, TYPE_WLAN),
        bluetooth_blocked: all_blocked(radios, TYPE_BLUETOOTH),
    }
}

/// Aplica un evento leído a las radios. Lo que no es de una radio (un
/// `CHANGE_ALL` no se lee nunca, el kernel lo convierte en `CHANGE`) se ignora.
pub fn apply_event(radios: &mut Radios, event: &RfkillEvent) {
    match event.op {
        Op::Add | Op::Change => {
            radios.insert(
                event.idx,
                Radio {
                    kind: event.kind,
                    soft: event.soft,
                    hard: event.hard,
                },
            );
        }
        Op::Del => {
            radios.remove(&event.idx);
        }
        Op::ChangeAll => {}
    }
}

/// Qué escribir para poner el modo, y qué anotar para quitarlo: las radios
/// que estaban encendidas (sin bloqueo de software).
///
/// Se bloquean **todos los tipos** con un solo `CHANGE_ALL` de [`TYPE_ALL`]:
/// también las que aparezcan mientras el modo está puesto nacen bloqueadas.
pub fn plan_enable(radios: &Radios) -> (Vec<RfkillEvent>, BTreeSet<u32>) {
    let on = radios
        .iter()
        .filter(|(_, radio)| !radio.soft)
        .map(|(idx, _)| *idx)
        .collect();
    (vec![RfkillEvent::change_all(TYPE_ALL, true)], on)
}

/// Qué escribir para quitar el modo.
///
/// Con la nota de [`plan_enable`], se desbloquea sólo lo que estaba encendido.
/// Para no dejar trabado el estado con que nacen las radios nuevas (lo puso el
/// `CHANGE_ALL` al entrar), un tipo cuyas radios estaban **todas** encendidas
/// —o que no tiene ninguna— se desbloquea entero con `CHANGE_ALL`; uno con
/// radios apagadas de antes, radio por radio.
///
/// Sin nota —el modo lo puso la tecla u otra herramienta— o con una nota que
/// no sirve (no queda ninguna de esas radios), se desbloquean todas.
pub fn plan_disable(radios: &Radios, saved: Option<&BTreeSet<u32>>) -> Vec<RfkillEvent> {
    let Some(saved) = saved.filter(|saved| saved.iter().any(|idx| radios.contains_key(idx))) else {
        return vec![RfkillEvent::change_all(TYPE_ALL, false)];
    };

    let mut kinds: BTreeSet<u8> = (1..NUM_TYPES).collect();
    kinds.extend(radios.values().map(|radio| radio.kind));

    let mut plan = Vec::new();
    for kind in kinds {
        let of_kind: Vec<u32> = radios
            .iter()
            .filter(|(_, radio)| radio.kind == kind)
            .map(|(idx, _)| *idx)
            .collect();
        if of_kind.iter().all(|idx| saved.contains(idx)) {
            plan.push(RfkillEvent::change_all(kind, false));
        } else {
            plan.extend(
                of_kind
                    .into_iter()
                    .filter(|idx| saved.contains(idx))
                    .map(|idx| RfkillEvent::change(idx, false)),
            );
        }
    }
    plan
}

#[derive(Debug, Default)]
struct Inner {
    radios: Radios,
    writable: bool,
    /// Las radios encendidas al poner el modo desde acá.
    saved: Option<BTreeSet<u32>>,
    state: AirplaneModeState,
}

/// La copia en memoria. Aparte de la global para que cada prueba tenga la suya.
#[derive(Debug, Default)]
pub struct Mirror {
    inner: Mutex<Inner>,
}

impl Mirror {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                radios: BTreeMap::new(),
                writable: false,
                saved: None,
                state: AirplaneModeState {
                    available: false,
                    enabled: false,
                    hardware: false,
                    wlan_blocked: false,
                    bluetooth_blocked: false,
                },
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // Nadie entra en pánico con el candado tomado; si pasara, lo que hay
        // adentro sigue siendo coherente.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn state(&self) -> AirplaneModeState {
        self.lock().state
    }

    /// Empieza de cero: el dispositivo se (re)abrió y va a contar todas las
    /// radios otra vez. Devuelve el estado si cambió.
    pub fn reset(&self, writable: bool) -> Option<AirplaneModeState> {
        let mut inner = self.lock();
        inner.radios.clear();
        inner.writable = writable;
        Self::settle(&mut inner)
    }

    /// Aplica una tanda de eventos y devuelve el estado si cambió. Una tanda
    /// —las radios al abrir, o los cambios de un `CHANGE_ALL`— avisa una vez.
    pub fn apply(&self, events: &[RfkillEvent]) -> Option<AirplaneModeState> {
        let mut inner = self.lock();
        for event in events {
            apply_event(&mut inner.radios, event);
        }
        Self::settle(&mut inner)
    }

    fn settle(inner: &mut Inner) -> Option<AirplaneModeState> {
        let next = derive(&inner.radios, inner.writable);
        let previous = std::mem::replace(&mut inner.state, next);
        // Se quitó el modo (desde acá o desde otro lado): la nota ya no vale.
        if previous.enabled && !next.enabled {
            inner.saved = None;
        }
        (previous != next).then_some(next)
    }

    /// Qué escribir para dejar el modo en `enabled`, y si ya estaba así. Si es
    /// para ponerlo, anota qué estaba encendido.
    pub fn plan(&self, enabled: bool) -> Result<(bool, Vec<RfkillEvent>), String> {
        let mut inner = self.lock();
        if inner.radios.is_empty() {
            return Err("el equipo no tiene radios".into());
        }
        let previous = inner.state.enabled;
        if previous == enabled {
            return Ok((previous, Vec::new()));
        }
        let plan = if enabled {
            let (plan, on) = plan_enable(&inner.radios);
            inner.saved = Some(on);
            plan
        } else {
            plan_disable(&inner.radios, inner.saved.as_ref())
        };
        Ok((previous, plan))
    }
}

static MIRROR: Mirror = Mirror::new();

/// Para publicar a las ventanas. Se pone al arrancar el seguimiento.
static APP: OnceLock<AppHandle> = OnceLock::new();

fn publish(state: AirplaneModeState) {
    if let Some(app) = APP.get() {
        let _ = app.emit(CHANGED_EVENT, state);
    }
}

/// El estado conocido, sin tocar el dispositivo.
pub fn state() -> AirplaneModeState {
    MIRROR.state()
}

/// Escribe los eventos en `out`, uno por `write(2)` como pide el kernel.
pub fn write_plan(out: &mut impl Write, plan: &[RfkillEvent]) -> io::Result<()> {
    for event in plan {
        out.write_all(&event.encode())?;
    }
    Ok(())
}

/// Pone o quita el modo en `mirror` escribiendo en `out`, y devuelve el que
/// había. El estado nuevo no se publica acá: llega por los eventos del
/// dispositivo, como cualquier otro cambio.
pub fn set_enabled_on(
    mirror: &Mirror,
    out: &mut impl Write,
    enabled: bool,
) -> Result<bool, String> {
    let (previous, plan) = mirror.plan(enabled)?;
    write_plan(out, &plan).map_err(|e| format!("/dev/rfkill no aceptó el cambio: {e}"))?;
    Ok(previous)
}

/// Pone o quita el modo avión y devuelve el estado anterior.
pub fn set_enabled(enabled: bool) -> Result<bool, String> {
    let mut device = OpenOptions::new()
        .write(true)
        .open(RFKILL_PATH)
        .map_err(|e| format!("no se pudo abrir {RFKILL_PATH} para escribir: {e}"))?;
    set_enabled_on(&MIRROR, &mut device, enabled)
}

/// El tipo de radio que nombra el frontend.
pub fn kind_from_name(name: &str) -> Option<u8> {
    match name {
        "wlan" => Some(TYPE_WLAN),
        "bluetooth" => Some(TYPE_BLUETOOTH),
        _ => None,
    }
}

/// Desbloquea todas las radios de un tipo. Es lo que hacen los mosaicos de
/// Wi-Fi y Bluetooth al tocarlos con la radio bloqueada: así no dependen de
/// que NetworkManager o BlueZ sepan sacar el bloqueo.
pub fn unblock(kind: u8) -> Result<(), String> {
    let mut device = OpenOptions::new()
        .write(true)
        .open(RFKILL_PATH)
        .map_err(|e| format!("no se pudo abrir {RFKILL_PATH} para escribir: {e}"))?;
    write_plan(&mut device, &[RfkillEvent::change_all(kind, false)])
        .map_err(|e| format!("/dev/rfkill no aceptó el cambio: {e}"))
}

/// Lee todos los eventos que haya sin bloquear. `Ok(false)` si se cerró.
fn drain(file: &File, events: &mut Vec<RfkillEvent>) -> io::Result<bool> {
    // Un evento por lectura: el kernel nunca entrega dos juntos.
    let mut buf = [0u8; 16];
    loop {
        match (&*file).read(&mut buf) {
            Ok(0) => return Ok(false),
            Ok(n) => events.extend(RfkillEvent::parse(&buf[..n])),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(true),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
}

/// Sigue los eventos de `file` (abierto sin bloqueo) hasta que se cierra.
/// `on_change` se llama una vez por tanda, y sólo si el estado cambió.
pub async fn follow(
    file: File,
    mirror: &Mirror,
    on_change: impl Fn(AirplaneModeState),
) -> io::Result<()> {
    let fd = AsyncFd::new(file)?;
    let mut events = Vec::new();
    loop {
        let mut ready = fd.readable().await?;
        events.clear();
        let open = drain(ready.get_inner(), &mut events)?;
        ready.clear_ready();
        if let Some(state) = mirror.apply(&events) {
            on_change(state);
        }
        if !open {
            return Ok(());
        }
    }
}

/// Abre `/dev/rfkill` sin bloqueo. Con escritura si se puede; si no, sólo
/// para leer (el modo se ve, pero no se puede cambiar).
fn open_device() -> io::Result<(File, bool)> {
    let open = |write: bool| {
        OpenOptions::new()
            .read(true)
            .write(write)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(RFKILL_PATH)
    };
    match open(true) {
        Ok(file) => Ok((file, true)),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            log_error(&format!(
                "Modo avión: sin permiso de escritura sobre {RFKILL_PATH} ({e}); se ve pero no se puede cambiar"
            ));
            Ok((open(false)?, false))
        }
        Err(e) => Err(e),
    }
}

/// Arranca el seguimiento. Si el dispositivo falla, se reintenta con espera
/// creciente; mientras tanto el modo se ve no disponible.
pub fn start(app: AppHandle) {
    let _ = APP.set(app);
    tauri::async_runtime::spawn(async move {
        let mut delay = Duration::from_secs(1);
        loop {
            let result = match open_device() {
                Ok((file, writable)) => {
                    if let Some(state) = MIRROR.reset(writable) {
                        publish(state);
                    }
                    log_info("Modo avión: siguiendo /dev/rfkill");
                    follow(file, &MIRROR, publish).await
                }
                Err(e) => Err(e),
            };
            if let Some(state) = MIRROR.reset(false) {
                publish(state);
            }
            match result {
                Ok(()) => delay = Duration::from_secs(1),
                Err(e) => {
                    log_error(&format!("Modo avión: no se pudo leer {RFKILL_PATH}: {e}"));
                    delay = (delay * 2).min(Duration::from_secs(300));
                }
            }
            tokio::time::sleep(delay).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixDatagram;

    fn radio(kind: u8, soft: bool) -> Radio {
        Radio {
            kind,
            soft,
            hard: false,
        }
    }

    fn event(idx: u32, kind: u8, op: Op, soft: bool, hard: bool) -> RfkillEvent {
        RfkillEvent {
            idx,
            kind,
            op,
            soft,
            hard,
        }
    }

    /// Lo que hace el kernel con lo que se le escribe (`rfkill_fop_write`):
    /// un `CHANGE_ALL` cambia todas las radios del tipo (o todas, con
    /// `TYPE_ALL`); un `CHANGE`, la de `idx`. El bloqueo por hardware no se
    /// toca.
    fn kernel(radios: &mut Radios, plan: &[RfkillEvent]) {
        for written in plan {
            for (idx, radio) in radios.iter_mut() {
                let hit = match written.op {
                    Op::ChangeAll => written.kind == TYPE_ALL || written.kind == radio.kind,
                    Op::Change => *idx == written.idx,
                    _ => false,
                };
                if hit {
                    radio.soft = written.soft;
                }
            }
        }
    }

    /// Wi-Fi encendido, Bluetooth apagado de antes, WWAN encendido.
    fn laptop() -> Radios {
        Radios::from([
            (0, radio(TYPE_WLAN, false)),
            (1, radio(TYPE_BLUETOOTH, true)),
            (2, radio(5, false)),
        ])
    }

    #[test]
    fn el_frontend_nombra_solo_wifi_y_bluetooth() {
        assert_eq!(kind_from_name("wlan"), Some(TYPE_WLAN));
        assert_eq!(kind_from_name("bluetooth"), Some(TYPE_BLUETOOTH));
        assert_eq!(kind_from_name("all"), None, "todo es cosa del modo avión");
    }

    #[test]
    fn lee_la_estructura_rfkill_event() {
        // idx 3 (en el orden del equipo), Bluetooth, CHANGE, soft, sin hard.
        let mut bytes = 3u32.to_ne_bytes().to_vec();
        bytes.extend([TYPE_BLUETOOTH, 2, 1, 0]);
        assert_eq!(
            RfkillEvent::parse(&bytes),
            Some(event(3, TYPE_BLUETOOTH, Op::Change, true, false))
        );
    }

    #[test]
    fn acepta_el_evento_extendido_de_nueve_bytes() {
        let mut bytes = 7u32.to_ne_bytes().to_vec();
        bytes.extend([TYPE_WLAN, 0, 0, 1, 1]);
        assert_eq!(
            RfkillEvent::parse(&bytes),
            Some(event(7, TYPE_WLAN, Op::Add, false, true))
        );
    }

    #[test]
    fn un_evento_corto_o_desconocido_no_es_evento() {
        assert_eq!(RfkillEvent::parse(&[0, 0, 0, 0, 1, 0, 0]), None);
        assert_eq!(RfkillEvent::parse(&[0, 0, 0, 0, 1, 9, 0, 0]), None);
    }

    #[test]
    fn lo_que_se_escribe_es_lo_que_se_lee() {
        let written = RfkillEvent::change(42, true);
        let bytes = written.encode();
        assert_eq!(bytes.len(), EVENT_SIZE);
        assert_eq!(RfkillEvent::parse(&bytes), Some(written));
        assert_eq!(
            RfkillEvent::change_all(TYPE_ALL, true).encode(),
            [0, 0, 0, 0, 0, 3, 1, 0],
            "CHANGE_ALL de todos los tipos, bloqueando"
        );
    }

    #[test]
    fn los_eventos_llevan_la_cuenta_de_las_radios() {
        let mut radios = Radios::new();
        apply_event(&mut radios, &event(0, TYPE_WLAN, Op::Add, false, false));
        apply_event(
            &mut radios,
            &event(1, TYPE_BLUETOOTH, Op::Add, false, false),
        );
        apply_event(
            &mut radios,
            &event(1, TYPE_BLUETOOTH, Op::Change, true, false),
        );
        assert_eq!(radios[&1], radio(TYPE_BLUETOOTH, true));
        apply_event(&mut radios, &event(0, TYPE_WLAN, Op::Del, false, false));
        assert_eq!(radios.len(), 1);
    }

    #[test]
    fn el_modo_esta_puesto_si_todas_estan_bloqueadas() {
        let mut radios = laptop();
        let state = derive(&radios, true);
        assert!(state.available && !state.enabled);
        assert!(!state.wlan_blocked && state.bluetooth_blocked);

        radios.get_mut(&0).unwrap().soft = true;
        radios.get_mut(&2).unwrap().hard = true;
        let state = derive(&radios, true);
        assert!(state.enabled, "bloqueada por hardware también cuenta");
        assert!(state.hardware && state.wlan_blocked);
    }

    #[test]
    fn sin_radios_no_esta_disponible() {
        let state = derive(&Radios::new(), true);
        assert_eq!(state, AirplaneModeState::default());
        assert!(
            Mirror::new().plan(true).is_err(),
            "no hay nada que bloquear"
        );
    }

    #[test]
    fn sin_permiso_de_escritura_no_esta_disponible() {
        let state = derive(&laptop(), false);
        assert!(!state.available);
    }

    #[test]
    fn ponerlo_bloquea_todos_los_tipos() {
        let mut radios = laptop();
        let (plan, on) = plan_enable(&radios);
        assert_eq!(plan, vec![RfkillEvent::change_all(TYPE_ALL, true)]);
        assert_eq!(on, BTreeSet::from([0, 2]), "anota las encendidas");
        kernel(&mut radios, &plan);
        assert!(radios.values().all(|r| r.soft));
    }

    #[test]
    fn quitarlo_restaura_solo_lo_que_estaba_encendido() {
        let before = laptop();
        let mut radios = before.clone();
        let (plan, on) = plan_enable(&radios);
        kernel(&mut radios, &plan);

        let plan = plan_disable(&radios, Some(&on));
        kernel(&mut radios, &plan);
        assert_eq!(
            radios, before,
            "Bluetooth sigue apagado; el resto, prendido"
        );
        assert!(
            !plan.contains(&RfkillEvent::change_all(TYPE_BLUETOOTH, false)),
            "el tipo con una radio apagada de antes no se desbloquea entero"
        );
        assert!(
            plan.contains(&RfkillEvent::change_all(TYPE_WLAN, false)),
            "el Wi-Fi, que estaba todo prendido, se desbloquea entero: una placa nueva no nace bloqueada"
        );
    }

    #[test]
    fn dos_radios_del_mismo_tipo_se_restauran_cada_una() {
        let before = Radios::from([
            (0, radio(TYPE_BLUETOOTH, false)),
            (1, radio(TYPE_BLUETOOTH, true)),
        ]);
        let mut radios = before.clone();
        let (plan, on) = plan_enable(&radios);
        kernel(&mut radios, &plan);
        let plan = plan_disable(&radios, Some(&on));
        assert!(plan.contains(&RfkillEvent::change(0, false)));
        assert!(!plan.contains(&RfkillEvent::change(1, false)));
        assert!(
            !plan.contains(&RfkillEvent::change_all(TYPE_BLUETOOTH, false)),
            "no se desbloquea el tipo entero: la otra radio estaba apagada"
        );
        assert!(
            plan.contains(&RfkillEvent::change_all(TYPE_WLAN, false)),
            "un tipo sin radios vuelve a nacer desbloqueado"
        );
        kernel(&mut radios, &plan);
        assert_eq!(radios, before);
    }

    #[test]
    fn sin_nota_se_desbloquean_todas() {
        // La tecla de avión lo puso: no hay qué estaba encendido.
        let radios = Radios::from([
            (0, radio(TYPE_WLAN, true)),
            (1, radio(TYPE_BLUETOOTH, true)),
        ]);
        assert_eq!(
            plan_disable(&radios, None),
            vec![RfkillEvent::change_all(TYPE_ALL, false)]
        );
        // Una nota de radios que ya no están tampoco sirve.
        assert_eq!(
            plan_disable(&radios, Some(&BTreeSet::from([9]))),
            vec![RfkillEvent::change_all(TYPE_ALL, false)]
        );
    }

    #[test]
    fn la_copia_avisa_una_vez_por_tanda() {
        let mirror = Mirror::new();
        assert_eq!(mirror.reset(true), None, "sin radios no cambia nada");
        let added = [
            event(0, TYPE_WLAN, Op::Add, false, false),
            event(1, TYPE_BLUETOOTH, Op::Add, false, false),
        ];
        let state = mirror.apply(&added).expect("aparecieron radios");
        assert!(state.available && !state.enabled);
        assert_eq!(mirror.apply(&added), None, "lo mismo otra vez no avisa");
    }

    #[test]
    fn quitarlo_desde_otro_lado_descarta_la_nota() {
        let mirror = Mirror::new();
        mirror.reset(true);
        mirror.apply(&[
            event(0, TYPE_WLAN, Op::Add, false, false),
            event(1, TYPE_BLUETOOTH, Op::Add, true, false),
        ]);
        let (previous, _) = mirror.plan(true).unwrap();
        assert!(!previous);
        mirror.apply(&[event(0, TYPE_WLAN, Op::Change, true, false)]);
        assert!(mirror.state().enabled);

        // Alguien prende el Bluetooth y lo vuelve a apagar: la nota ya no dice
        // la verdad, así que no se usa.
        mirror.apply(&[event(1, TYPE_BLUETOOTH, Op::Change, false, false)]);
        mirror.apply(&[event(1, TYPE_BLUETOOTH, Op::Change, true, false)]);
        let (_, plan) = mirror.plan(false).unwrap();
        assert_eq!(plan, vec![RfkillEvent::change_all(TYPE_ALL, false)]);
    }

    #[test]
    fn poner_lo_que_ya_esta_no_escribe_nada() {
        let mirror = Mirror::new();
        mirror.reset(true);
        mirror.apply(&[event(0, TYPE_WLAN, Op::Add, false, false)]);
        let mut out = Vec::new();
        assert!(!set_enabled_on(&mirror, &mut out, false).unwrap());
        assert!(out.is_empty());
        assert!(!set_enabled_on(&mirror, &mut out, true).unwrap());
        assert_eq!(out, RfkillEvent::change_all(TYPE_ALL, true).encode());
    }

    /// El seguimiento lee los eventos como los entrega el kernel —uno por
    /// lectura— de un fd sin bloqueo, sin sondear: un socket de datagramas
    /// hace de `/dev/rfkill`.
    #[tokio::test]
    async fn el_seguimiento_se_entera_por_los_eventos() {
        let (kernel_side, ours) = UnixDatagram::pair().unwrap();
        ours.set_nonblocking(true).unwrap();
        let file = File::from(OwnedFd::from(ours));
        let mirror = std::sync::Arc::new(Mirror::new());
        mirror.reset(true);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let task = {
            let mirror = mirror.clone();
            tokio::spawn(async move {
                follow(file, &mirror, move |state| {
                    let _ = tx.send(state);
                })
                .await
            })
        };

        // Al abrir, un ADD por radio: una sola tanda, un solo aviso.
        kernel_side
            .send(&event(0, TYPE_WLAN, Op::Add, false, false).encode())
            .unwrap();
        kernel_side
            .send(&event(1, TYPE_BLUETOOTH, Op::Add, false, false).encode())
            .unwrap();
        let first = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(first.available);

        // La tecla de avión: el kernel bloquea las dos.
        kernel_side
            .send(&event(0, TYPE_WLAN, Op::Change, true, false).encode())
            .unwrap();
        kernel_side
            .send(&event(1, TYPE_BLUETOOTH, Op::Change, true, false).encode())
            .unwrap();
        let mut last = first;
        while !last.enabled {
            last = tokio::time::timeout(Duration::from_secs(5), rx.recv())
                .await
                .expect("llega el cambio")
                .unwrap();
        }
        assert!(mirror.state().enabled);

        task.abort();
    }

    /// Medición, a mano y con el dispositivo de verdad: `cargo test --lib
    /// airplane_mode::tests::medir -- --ignored --nocapture`. En reposo, el
    /// seguimiento avisa una vez (las radios al abrir) y no vuelve a leer; y
    /// preguntar el estado cuesta lo que tomar un candado.
    #[tokio::test]
    #[ignore = "usa /dev/rfkill de la máquina"]
    async fn medir_el_seguimiento_en_reposo() {
        let (file, _) = open_device().expect("se abre /dev/rfkill");
        let mirror = std::sync::Arc::new(Mirror::new());
        mirror.reset(true);
        let changes = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let task = {
            let mirror = mirror.clone();
            let changes = changes.clone();
            tokio::spawn(async move {
                let _ = follow(file, &mirror, move |_| {
                    changes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                })
                .await;
            })
        };
        tokio::time::sleep(Duration::from_secs(5)).await;
        task.abort();
        println!(
            "avisos en 5 s de reposo: {}",
            changes.load(std::sync::atomic::Ordering::SeqCst)
        );
        println!("estado: {:?}", mirror.state());

        let start = std::time::Instant::now();
        for _ in 0..1_000_000 {
            std::hint::black_box(mirror.state());
        }
        println!(
            "preguntar el estado: {:?} por llamada",
            start.elapsed() / 1_000_000
        );
    }
}
