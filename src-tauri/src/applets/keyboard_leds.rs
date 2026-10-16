use super::Applet;
use async_trait::async_trait;
use serde_json::json;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{fs, io};
use std::os::unix::io::AsRawFd;
use tauri::{AppHandle, Emitter, Manager};

use crate::windows_apps::create_osd_window;

// ─── C a p s   L o c k   s t a t e ───────────────────────────────────────────

/// OSD icon and translation key for a Caps Lock state.
///
/// Pure on purpose: the mapping is what the frontend contract is made of, so it
/// is the part worth testing without a compositor around.
fn caps_indicator(active: bool) -> (&'static str, &'static str) {
    if active {
        ("capslock-enabled-symbolic", "osd.capsLockOn")
    } else {
        ("capslock-disabled-symbolic", "osd.capsLockOff")
    }
}

/// The OSD bar value for a Caps Lock state (it is a toggle, so 1 or 0).
fn caps_osd_value(active: bool) -> f64 {
    if active {
        1.0
    } else {
        0.0
    }
}

/// Tells the panel and the OSD about a Caps Lock state.
///
/// `initial` marks the announcement made at startup: the panel icon always has
/// to be synced, but popping an OSD just because the session began with Caps
/// Lock off would be noise, so only the "on" case shows one — same behaviour as
/// before.
fn announce_caps_state(app: &AppHandle, active: bool, initial: bool) {
    let (icon, label) = caps_indicator(active);

    let _ = app.emit("caps-lock-changed", json!({ "active": active }));

    if !initial || active {
        show_osd_sync(icon, caps_osd_value(active), 1.0, label, app);
    }
}

// ─── C a p s   L o c k   v i a   G D K  ( c o m p o s i t o r ) ──────────────

/// Follows Caps Lock through GDK's keymap, which mirrors the compositor's xkb
/// state.
///
/// **Must run on the GTK main thread** — GDK is not thread safe and the signal
/// has to be connected where the main loop that dispatches it lives.
///
/// Returns `false` when there is no display or no keymap to ask, so the caller
/// Sigue el estado de Bloq Mayús por las luces del kernel.
///
/// Antes esto preguntaba por GDK, que es quien tiene el estado de xkb del
/// compositor. No sirve: GDK se entera de los modificadores por los eventos que
/// el compositor manda **a la ventana con foco de teclado**, y el panel es una
/// superficie de capa que nunca lo toma. Medido en esta máquina: escribiendo en
/// mayúsculas, GDK seguía diciendo que Bloq Mayús estaba apagado.
fn start_caps_monitor(app: AppHandle, running: Arc<AtomicBool>) {
    spawn_evdev_caps_monitor(app, running);
}

// ─── L a s   l u c e s   d e l   t e c l a d o ───────────────────────────────
// Se leen del kernel por udev uaccess: sin grupo `input`, sin IPC del
// compositor, sin FIFO.
//
// Se miran **todos** los teclados que se puedan abrir, no uno solo. En Wayland
// el compositor enciende la luz por dispositivo, y cuando otro proceso toma el
// teclado en exclusiva y reinyecta por un teclado virtual —que es exactamente
// lo que hace `vasak-press-and-hold` para el selector de acentos— la única luz
// que se mueve es la del virtual, que no aparece en `/dev/input/by-path`.
// Mirando uno solo, además, un teclado externo era invisible.

const EV_KEY: u16 = 0x01;
const EV_LED: u16 = 0x11;
const KEY_CAPSLOCK: u16 = 58;

/// **1**, no 0: el cero es Bloq Num.
///
/// Estaba puesto en 0, así que lo que el indicador mostraba era el Bloq Num —que
/// en un teclado con teclado numérico vive encendido—: de ahí que pareciera
/// trabado en «mayúsculas activadas» sin importar lo que uno apretara.
const LED_CAPSL: u16 = 1;

// EVIOCGLED = _IOR('E', 0x19, int) = 0x80044519 (64-bit)
const EVIOCGLED: libc::c_ulong = 0x8004_4519;

/// EVIOCGBIT(EV_LED, 1) = _IOC(_IOC_READ, 'E', 0x20 + EV_LED, 1): qué luces dice
/// tener el dispositivo. Es lo que separa un teclado de un mouse o del botón de
/// encendido, que también son `/dev/input/event*`.
const EVIOCGBIT_LED: libc::c_ulong = 0x8001_4531;

#[repr(C)]
struct InputEvent {
    _tv_sec: i64,
    _tv_usec: i64,
    type_: u16,
    code: u16,
    value: i32,
}

/// Si el dispositivo dice tener luz de Bloq Mayús.
fn tiene_luz_de_mayusculas(fd: std::os::unix::io::RawFd) -> bool {
    let mut luces: u8 = 0;
    let leidos = unsafe {
        libc::ioctl(fd, EVIOCGBIT_LED, &mut luces as *mut u8 as *mut libc::c_void)
    };

    leidos >= 0 && (luces >> LED_CAPSL) & 1 == 1
}

/// Todos los teclados que se pueden abrir y tienen esa luz.
///
/// Se recorre `/dev/input/event*` y no `/dev/input/by-path`: el teclado virtual
/// que crea el selector de acentos —el único cuya luz se mueve mientras tiene
/// tomado el teclado de verdad— no tiene entrada ahí.
fn find_caps_devices() -> Vec<std::path::PathBuf> {
    let mut encontrados = Vec::new();

    let Ok(dir) = fs::read_dir("/dev/input") else {
        log::warn!("No se pudo leer /dev/input: Bloq Mayús queda sin seguimiento");
        return encontrados;
    };

    for entrada in dir.flatten() {
        let ruta = entrada.path();
        let es_evento = ruta
            .file_name()
            .and_then(|nombre| nombre.to_str())
            .is_some_and(|nombre| nombre.starts_with("event"));

        if !es_evento {
            continue;
        }

        // Abrir es también la prueba de permisos: sin uaccess no hay nada que
        // hacer con este dispositivo.
        let Ok(archivo) = fs::File::open(&ruta) else {
            continue;
        };

        if tiene_luz_de_mayusculas(archivo.as_raw_fd()) {
            encontrados.push(ruta);
        }
    }

    if encontrados.is_empty() {
        log::warn!("Ningún teclado con luz de Bloq Mayús: el indicador queda sin seguimiento");
    }

    encontrados
}

fn caps_led_state(fd: std::os::unix::io::RawFd) -> bool {
    let mut luces: u32 = 0;
    unsafe {
        libc::ioctl(fd, EVIOCGLED, &mut luces as *mut u32 as *mut libc::c_void);
    }
    (luces >> LED_CAPSL) & 1 == 1
}

/// Un hilo por teclado, y un solo aviso por cambio.
///
/// Varios dispositivos pueden reportar la misma tecla —el físico y el virtual
/// que lo replica—, así que el último estado avisado se comparte entre los
/// hilos: el segundo que llega con la misma novedad se calla.
fn spawn_evdev_caps_monitor(app: AppHandle, running: Arc<AtomicBool>) {
    let dispositivos = find_caps_devices();
    if dispositivos.is_empty() {
        return;
    }

    // El estado inicial sale del primero que se pueda leer, y se avisa una sola
    // vez, antes de largar los hilos.
    let inicial = dispositivos
        .iter()
        .find_map(|ruta| fs::File::open(ruta).ok())
        .map(|archivo| caps_led_state(archivo.as_raw_fd()))
        .unwrap_or(false);

    log::info!("Estado inicial de Bloq Mayús: {inicial}");
    announce_caps_state(&app, inicial, true);

    let ultimo = Arc::new(AtomicBool::new(inicial));

    for ruta in dispositivos {
        let app = app.clone();
        let running = running.clone();
        let ultimo = Arc::clone(&ultimo);

        std::thread::spawn(move || {
            log::info!("Siguiendo Bloq Mayús en {}", ruta.display());

            let Ok(mut archivo) = fs::File::open(&ruta) else {
                log::error!("No se pudo abrir {} para leer sus eventos", ruta.display());
                return;
            };

            let fd = archivo.as_raw_fd();
            let mut buf = [0u8; 24];

            while running.load(Ordering::Relaxed) {
                if let Err(error) = io::Read::read_exact(&mut archivo, &mut buf) {
                    // Un teclado que se desenchufa cierra su hilo y ya; los
                    // demás siguen andando.
                    log::info!("Se dejó de leer {}: {error}", ruta.display());
                    break;
                }

                let evento: InputEvent = unsafe { std::mem::transmute(buf) };

                let nuevo = match (evento.type_, evento.code, evento.value) {
                    (EV_LED, LED_CAPSL, valor) => Some(valor != 0),
                    // Al soltar la tecla el kernel ya movió la luz; preguntarla
                    // cubre a los teclados que no emiten el evento de LED.
                    (EV_KEY, KEY_CAPSLOCK, 0) => Some(caps_led_state(fd)),
                    _ => None,
                };

                let Some(nuevo) = nuevo else {
                    continue;
                };

                // `swap` y no `load`+`store`: dos teclados pueden traer el mismo
                // cambio al mismo tiempo y sólo uno tiene que avisar.
                if ultimo.swap(nuevo, Ordering::SeqCst) == nuevo {
                    continue;
                }

                log::info!("Bloq Mayús cambió: {nuevo}");
                announce_caps_state(&app, nuevo, false);
            }
        });
    }
}

fn show_osd_sync(icon: &str, value: f64, maximum: f64, label: &str, app: &AppHandle) {
    if let Some(window) = app.get_webview_window("osd_popup") {
        let _ = window.emit(
            "osd:show",
            json!({
                "icon": icon,
                "value": value,
                "maximum": maximum,
                "label": label,
            }),
        );
        let _ = window.show();
        let _ = window.set_focus();
    }
}

// ─── M i c   m u t e   v i a   P u l s e A u d i o ───────────────────────────

/// Reads the current microphone mute state.
async fn read_mic_muted() -> Option<bool> {
    let output = tokio::process::Command::new("pactl")
        .args(["get-source-mute", "@DEFAULT_SOURCE@"])
        .output()
        .await
        .ok()?;

    String::from_utf8(output.stdout)
        .ok()
        .map(|text| text.trim().contains("yes"))
}

async fn announce_mic_state(app: &AppHandle, muted: bool) {
    let (label, icon) = if muted {
        ("osd.micMuted", "microphone-sensitivity-muted")
    } else {
        ("osd.micActive", "microphone-sensitivity-high")
    };

    let _ = app.emit("mic-mute-changed", json!({ "active": muted }));
    let _ = crate::commands::osd::show_osd_internal(
        icon,
        if muted { 1.0 } else { 0.0 },
        1.0,
        label,
        app,
    )
    .await;
}

/// Follows the microphone mute state through `pactl subscribe`.
///
/// This used to fork a `pactl get-source-mute` process **every second** for the
/// whole session — around 86,000 processes a day — to notice a change that
/// happens a handful of times. `pactl subscribe` is a single long-lived process
/// that prints a line when audio state changes, so the state is only queried
/// when something actually happened.
fn spawn_mic_monitor(app: AppHandle, running: Arc<AtomicBool>) {
    tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, BufReader};
        use tokio::process::Command;
        use std::process::Stdio;

        let mut last_muted: Option<bool> = read_mic_muted().await;
        let mut backoff = std::time::Duration::from_secs(1);

        while running.load(Ordering::Relaxed) {
            let child = Command::new("pactl")
                .arg("subscribe")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn();

            let mut child = match child {
                Ok(child) => child,
                Err(error) => {
                    log::error!("No se pudo iniciar `pactl subscribe`: {error}");
                    tokio::time::sleep(backoff).await;
                    // Back off so a missing PulseAudio doesn't become a spin loop.
                    backoff = (backoff * 2).min(std::time::Duration::from_secs(30));
                    continue;
                }
            };

            backoff = std::time::Duration::from_secs(1);

            let Some(stdout) = child.stdout.take() else {
                let _ = child.kill().await;
                continue;
            };

            let mut lines = BufReader::new(stdout).lines();

            while running.load(Ordering::Relaxed) {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        // Only source events can change the microphone.
                        if !line.contains("source") {
                            continue;
                        }

                        let Some(muted) = read_mic_muted().await else {
                            continue;
                        };

                        if last_muted == Some(muted) {
                            continue;
                        }
                        last_muted = Some(muted);

                        announce_mic_state(&app, muted).await;
                    }
                    // PulseAudio went away; reconnect rather than going deaf.
                    Ok(None) | Err(_) => break,
                }
            }

            let _ = child.kill().await;
        }
    });
}

// ─── A p p l e t   t r a i t ──────────────────────────────────────────────────

#[async_trait]
impl Applet for KeyboardLedsApplet {
    fn name(&self) -> &'static str {
        "keyboard_leds"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        log::info!("Keyboard LEDs applet starting (GDK keymap + PulseAudio)");

        // Pre-create OSD window so sync dispatch can use it
        let _ = create_osd_window(&app, "capslock-disabled-symbolic", 0.0, 1.0, "").await;

        let running = Arc::new(AtomicBool::new(true));

        start_caps_monitor(app.clone(), running.clone());
        spawn_mic_monitor(app, running);

        Ok(())
    }
}

pub struct KeyboardLedsApplet;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_on_uses_the_enabled_icon_and_label() {
        assert_eq!(
            caps_indicator(true),
            ("capslock-enabled-symbolic", "osd.capsLockOn")
        );
    }

    #[test]
    fn caps_off_uses_the_disabled_icon_and_label() {
        assert_eq!(
            caps_indicator(false),
            ("capslock-disabled-symbolic", "osd.capsLockOff")
        );
    }

    #[test]
    fn osd_value_is_a_toggle_within_its_maximum() {
        assert_eq!(caps_osd_value(true), 1.0);
        assert_eq!(caps_osd_value(false), 0.0);
    }

    #[test]
    fn each_state_maps_to_a_distinct_indicator() {
        assert_ne!(caps_indicator(true), caps_indicator(false));
    }
}
