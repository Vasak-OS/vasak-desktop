//! Si hay alguien: la pantalla bloqueada y la inactividad.
//!
//! # Bloqueo
//!
//! La pantalla de bloqueo es otro proceso con una superficie
//! `ext-session-lock`, y el escritorio no la ve: la ventana que estaba
//! enfocada sigue «activada» para el compositor detrás del bloqueo. Lo que sí
//! se ve es el proceso. Se busca en `/proc` un programa de bloqueo conocido
//! —`vasak-lock-screen`, el de VasakOS, y los de siempre por si alguien cambió
//! el suyo— por su nombre de proceso (`comm`, que el núcleo corta a 15
//! caracteres: `vasak-lock-scre`). Es la misma forma con que el applet de
//! privacidad recorre `/proc`, y cuesta un `read_dir` por vuelta.
//!
//! # Inactividad
//!
//! Con `ext-idle-notify-v1`, el protocolo estándar de Wayland: se le pide al
//! compositor un aviso después de [`IDLE_THRESHOLD`] sin teclado ni puntero, y
//! él avisa cuando pasa y cuando vuelve la actividad. Se usa
//! `get_idle_notification` y no `get_input_idle_notification`: el primero
//! respeta a quien inhibe la inactividad, así que mirar una película a
//! pantalla completa sin tocar nada sigue contando, que es lo que la persona
//! espera. No es un protocolo privilegiado (no está en `permisos-globales`).
//! Si el compositor no lo ofrece, no hay detección de inactividad —se avisa
//! una vez en el registro— y el resto funciona igual.

use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Duration;

use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1, ext_idle_notifier_v1,
};

use crate::logger::{log_info, log_warning};

/// Cuánto sin teclado ni puntero cuenta como «no hay nadie».
///
/// Tres minutos: lo bastante largo para leer una página o pensar una frase sin
/// tocar nada —eso es usar la computadora—, y lo bastante corto para que irse
/// a hacer otra cosa no sume más que eso, que además se descuenta (ver
/// `tracker.rs`). El bloqueo automático de la sesión llega a los cinco
/// (`vasak-idle.service`), así que siempre se deja de contar antes.
pub const IDLE_THRESHOLD: Duration = Duration::from_secs(180);

/// Los programas de bloqueo que se reconocen, como los nombra `comm`.
const LOCKERS: [&str; 5] = [
    "vasak-lock-screen",
    "gtklock",
    "swaylock",
    "hyprlock",
    "waylock",
];

/// `comm` corta el nombre del programa a 15 caracteres.
const COMM_LENGTH: usize = 15;

/// Si `comm` (lo que dice `/proc/<pid>/comm`) es un programa de bloqueo.
pub fn is_locker(comm: &str) -> bool {
    let comm = comm.trim();
    !comm.is_empty()
        && LOCKERS.iter().any(|locker| {
            let cut: String = locker.chars().take(COMM_LENGTH).collect();
            comm == cut
        })
}

/// Si hay un programa de bloqueo corriendo, mirando `proc` (`/proc`).
pub fn screen_locked_in(proc: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(proc) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let is_process = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.bytes().all(|b| b.is_ascii_digit()));
        is_process
            && std::fs::read_to_string(entry.path().join("comm")).is_ok_and(|comm| is_locker(&comm))
    })
}

pub fn screen_locked() -> bool {
    screen_locked_in(Path::new("/proc"))
}

/// Lo que avisa el compositor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleEvent {
    Idle,
    Active,
}

struct IdleState {
    tx: Sender<IdleEvent>,
    notifier: Option<ext_idle_notifier_v1::ExtIdleNotifierV1>,
    seat: Option<wl_seat::WlSeat>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for IdleState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name, interface, ..
        } = event
        {
            match interface.as_str() {
                "ext_idle_notifier_v1" if state.notifier.is_none() => {
                    state.notifier = Some(registry.bind(name, 1, qh, ()));
                }
                "wl_seat" if state.seat.is_none() => {
                    state.seat = Some(registry.bind(name, 1, qh, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for IdleState {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ext_idle_notifier_v1::ExtIdleNotifierV1, ()> for IdleState {
    fn event(
        _: &mut Self,
        _: &ext_idle_notifier_v1::ExtIdleNotifierV1,
        _: ext_idle_notifier_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, ()> for IdleState {
    fn event(
        state: &mut Self,
        _: &ext_idle_notification_v1::ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let message = match event {
            ext_idle_notification_v1::Event::Idled => IdleEvent::Idle,
            ext_idle_notification_v1::Event::Resumed => IdleEvent::Active,
            _ => return,
        };
        let _ = state.tx.send(message);
    }
}

/// Arranca el hilo que escucha la inactividad y manda cada aviso por `tx`.
///
/// Una conexión de Wayland propia, aparte de la de GTK: así el hilo bloquea
/// esperando eventos sin tocar el bucle principal. Si no hay compositor o no
/// ofrece el protocolo, avisa en el registro y termina; el registro de tiempo
/// sigue, sin descontar inactividad.
pub fn watch_idle(tx: Sender<IdleEvent>) {
    let spawned = std::thread::Builder::new()
        .name("screen-time-idle".into())
        .spawn(move || {
            if let Err(error) = run_idle(tx) {
                log_warning(&format!(
                    "[tiempo de pantalla] sin detección de inactividad: {error}"
                ));
            }
        });
    if let Err(error) = spawned {
        log_warning(&format!(
            "[tiempo de pantalla] no se pudo arrancar el hilo de inactividad: {error}"
        ));
    }
}

fn run_idle(tx: Sender<IdleEvent>) -> Result<(), String> {
    let connection = Connection::connect_to_env().map_err(|error| error.to_string())?;
    let mut queue = connection.new_event_queue();
    let qh = queue.handle();
    connection.display().get_registry(&qh, ());

    let mut state = IdleState {
        tx,
        notifier: None,
        seat: None,
    };
    queue
        .roundtrip(&mut state)
        .map_err(|error| error.to_string())?;

    let (Some(notifier), Some(seat)) = (state.notifier.clone(), state.seat.clone()) else {
        return Err("el compositor no ofrece ext_idle_notifier_v1".into());
    };
    let timeout = u32::try_from(IDLE_THRESHOLD.as_millis()).unwrap_or(u32::MAX);
    let _notification = notifier.get_idle_notification(timeout, &seat, &qh, ());
    log_info(&format!(
        "[tiempo de pantalla] inactividad a los {} s",
        IDLE_THRESHOLD.as_secs()
    ));

    loop {
        queue
            .blocking_dispatch(&mut state)
            .map_err(|error| error.to_string())?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// Un directorio propio en el temporal del sistema: **nunca** `/proc` de
    /// verdad. Se borra al soltarlo. (Antes vivía en `store.rs`, que se fue con
    /// la contabilidad al servicio de salud.)
    pub struct TempDir(pub PathBuf);

    impl TempDir {
        pub fn new(label: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "vasak-screen-time-{label}-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4()
            ));
            fs::create_dir_all(&dir).expect("se crea el directorio temporal");
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn el_bloqueo_se_reconoce_por_el_nombre_cortado_del_proceso() {
        // `comm` corta a 15: así se ve el de VasakOS.
        assert!(is_locker("vasak-lock-scre\n"));
        assert!(is_locker("gtklock"));
        assert!(is_locker("swaylock"));
        assert!(!is_locker("vasak-lock"));
        assert!(!is_locker("firefox"));
        assert!(!is_locker(""));
    }

    #[test]
    fn con_un_programa_de_bloqueo_en_proc_la_pantalla_está_bloqueada() {
        let proc = TempDir::new("proc");
        for (pid, comm) in [
            ("1", "systemd"),
            ("4242", "firefox"),
            ("self", "vasak-lock-scre"),
        ] {
            fs::create_dir_all(proc.0.join(pid)).expect("se crea");
            fs::write(proc.0.join(pid).join("comm"), format!("{comm}\n")).expect("se escribe");
        }
        // `self` no es un pid: no cuenta.
        assert!(!screen_locked_in(&proc.0));

        fs::create_dir_all(proc.0.join("9001")).expect("se crea");
        fs::write(proc.0.join("9001/comm"), "vasak-lock-scre\n").expect("se escribe");
        assert!(screen_locked_in(&proc.0));

        assert!(!screen_locked_in(&proc.0.join("no-existe")));
    }

    #[test]
    fn el_umbral_de_inactividad_es_anterior_al_bloqueo_automático() {
        assert_eq!(IDLE_THRESHOLD, Duration::from_secs(180));
        assert!(IDLE_THRESHOLD < Duration::from_secs(300));
    }
}
