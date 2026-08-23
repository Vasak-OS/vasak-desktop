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
/// can fall back instead of leaving the indicator mute.
fn connect_gdk_caps_monitor(app: AppHandle) -> bool {
    let Some(display) = gdk::Display::default() else {
        log::warn!("Sin GDK display: no se puede seguir Bloq Mayús por el compositor");
        return false;
    };

    let Some(keymap) = gdk::Keymap::for_display(&display) else {
        log::warn!("Sin GDK keymap: no se puede seguir Bloq Mayús por el compositor");
        return false;
    };

    let initial = keymap.is_caps_locked();
    log::info!("Estado inicial de Bloq Mayús (GDK): {}", initial);
    announce_caps_state(&app, initial, true);

    // `state-changed` fires for every modifier (Num Lock, Scroll Lock,
    // direction…), so remember what we last reported and stay quiet otherwise.
    let last = std::cell::Cell::new(initial);

    keymap.connect_state_changed(move |keymap| {
        let active = keymap.is_caps_locked();
        if active == last.get() {
            return;
        }
        last.set(active);

        log::info!("Bloq Mayús cambió (GDK): {}", active);
        announce_caps_state(&app, active, false);
    });

    // The keymap wrapper is dropped here, but GDK keeps its own reference to the
    // display's keymap for the life of the display, so the handler stays live.
    true
}

/// Starts Caps Lock monitoring, preferring the compositor over the kernel LED.
///
/// GDK has to be touched from the main thread, and only there can we find out
/// whether a keymap exists at all — so the fallback decision is made inside the
/// main-thread closure, right where the answer is known.
fn start_caps_monitor(app: AppHandle, running: Arc<AtomicBool>) {
    let monitored = app.clone();
    let monitored_running = running.clone();

    let dispatch = app.run_on_main_thread(move || {
        if !connect_gdk_caps_monitor(monitored.clone()) {
            log::warn!("Cayendo al LED del kernel para Bloq Mayús (puede quedarse pegado)");
            spawn_evdev_caps_monitor(monitored, monitored_running);
        }
    });

    if let Err(error) = dispatch {
        log::error!(
            "No se pudo llegar al hilo principal para GDK ({error}); usando el LED del kernel"
        );
        spawn_evdev_caps_monitor(app, running);
    }
}

// ─── e v d e v   C a p s   L o c k   m o n i t o r  ( f a l l b a c k ) ──────
// Reads the keyboard LED straight from the kernel through udev uaccess — no
// input group, no Wayfire IPC, no FIFO.
//
// Only a fallback now: on Wayland the compositor owns that LED **per device**,
// so as soon as another process grabs the keyboard with EVIOCGRAB and replays
// keys through a uinput virtual keyboard — which is exactly what
// `vasak-press-and-hold` does for the accent picker — the compositor starts
// lighting the LED of the virtual device and the physical keyboard's LED stays
// on for good. It also only ever watched a single device, so an external
// keyboard was invisible to it.

const EV_KEY: u16 = 0x01;
const EV_LED: u16 = 0x11;
const KEY_CAPSLOCK: u16 = 58;
const LED_CAPSL: u16 = 0;
// EVIOCGLED = _IOR('E', 0x19, int) = 0x80044519 (64-bit)
const EVIOCGLED: libc::c_ulong = 0x8004_4519;

#[repr(C)]
struct InputEvent {
    _tv_sec: i64,
    _tv_usec: i64,
    type_: u16,
    code: u16,
    value: i32,
}

fn find_keyboard_evdev() -> Option<String> {
    if let Ok(dir) = fs::read_dir("/dev/input/by-path") {
        for entry in dir.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name()?.to_str() {
                if name.ends_with("-event-kbd") {
                    return Some(path.to_string_lossy().to_string());
                }
            }
        }
    }
    log::warn!("No keyboard evdev device found — Caps Lock monitoring unavailable");
    None
}

fn caps_led_state(fd: std::os::unix::io::RawFd) -> bool {
    let mut leds: u32 = 0;
    unsafe {
        libc::ioctl(fd, EVIOCGLED, &mut leds as *mut u32 as *mut libc::c_void);
    }
    (leds & 1) != 0
}

fn spawn_evdev_caps_monitor(app: AppHandle, running: Arc<AtomicBool>) {
    let device_path = match find_keyboard_evdev() {
        Some(p) => p,
        None => return,
    };

    std::thread::spawn(move || {
        log::info!("Caps Lock evdev monitor on {}", device_path);

        let mut file = match fs::File::open(&device_path) {
            Ok(f) => f,
            Err(e) => {
                log::error!("Cannot open evdev device {}: {}", device_path, e);
                log::warn!("Ensure udev uaccess grants access to input devices");
                return;
            }
        };

        let fd = file.as_raw_fd();

        // Read initial Caps Lock LED state from kernel
        let mut caps_state = caps_led_state(fd);
        log::info!("Initial Caps Lock state: {}", caps_state);

        announce_caps_state(&app, caps_state, true);

        let mut buf = [0u8; 24];

        loop {
            if !running.load(Ordering::Relaxed) {
                break;
            }

            if let Err(e) = io::Read::read_exact(&mut file, &mut buf) {
                log::error!("evdev read error: {}", e);
                break;
            }

            let event: InputEvent = unsafe { std::mem::transmute(buf) };

            let new_state = match (event.type_, event.code, event.value) {
                (EV_LED, LED_CAPSL, v) => Some(v != 0),
                (EV_KEY, KEY_CAPSLOCK, 1) => Some(caps_led_state(fd)),
                _ => None,
            };

            if let Some(new) = new_state {
                if new == caps_state {
                    continue;
                }
                caps_state = new;

                announce_caps_state(&app, caps_state, false);
                log::info!("Caps Lock state changed: {}", caps_state);
            }
        }
    });
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
