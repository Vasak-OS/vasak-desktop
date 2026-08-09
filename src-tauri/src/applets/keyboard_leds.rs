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

// ─── e v d e v   C a p s   L o c k   m o n i t o r ───────────────────────────
// Uses udev uaccess — no input group, no Wayfire IPC, no FIFO.

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

        let _ = app.emit("caps-lock-changed", json!({ "active": caps_state }));
        if caps_state {
            show_osd_sync("capslock-enabled-symbolic", 1.0, 1.0, "osd.capsLockOn", &app);
        }

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

                let label = if caps_state { "osd.capsLockOn" } else { "osd.capsLockOff" };
                let icon = if caps_state { "capslock-enabled-symbolic" } else { "capslock-disabled-symbolic" };

                let _ = app.emit("caps-lock-changed", json!({ "active": caps_state }));
                show_osd_sync(icon, if caps_state { 1.0 } else { 0.0 }, 1.0, label, &app);
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
        log::info!("Keyboard LEDs applet starting (evdev uaccess + PulseAudio)");

        // Pre-create OSD window so sync dispatch can use it
        let _ = create_osd_window(&app, "capslock-disabled-symbolic", 0.0, 1.0, "").await;

        let running = Arc::new(AtomicBool::new(true));

        spawn_evdev_caps_monitor(app.clone(), running.clone());
        spawn_mic_monitor(app, running);

        Ok(())
    }
}

pub struct KeyboardLedsApplet;
