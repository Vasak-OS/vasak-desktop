use super::Applet;
use async_trait::async_trait;
use serde_json::json;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{fs, io};
use tauri::{AppHandle, Emitter, Manager};

use crate::window_manager::wayfire_ipc::get_wayfire_client;
use crate::windows_apps::create_osd_window;

// ─── F I F O   p a t h ───────────────────────────────────────────────────────

fn fifo_path() -> String {
    let runtime = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::getuid() }));
    format!("{}/vasak-hotkey.fifo", runtime)
}

// ─── W a y f i r e   b i n d i n g   r e g i s t r a t i o n ─────────────────

async fn register_wayfire_binding(
    _app: AppHandle,
    fifo: String,
) {
    let client = match get_wayfire_client().await {
        Some(c) => c,
        None => {
            log::warn!("Cannot connect to Wayfire IPC — Caps Lock hotkey binding not registered");
            return;
        }
    };

    let command = format!("bash -c 'echo capslock > {}'", fifo);

    match client
        .send_and_wait(
            "command/register-binding",
            json!({
                "binding": "KEY_CAPSLOCK",
                "command": command,
            }),
        )
        .await
    {
        Ok(resp) => log::info!("Wayfire KEY_CAPSLOCK binding registered: {}", resp),
        Err(e) => log::error!("Failed to register KEY_CAPSLOCK binding: {}", e),
    }
}

// ─── F I F O   r e a d e r ───────────────────────────────────────────────────

fn spawn_fifo_reader(fifo: String, app: AppHandle, running: Arc<AtomicBool>, mut caps_state: bool) {
    std::thread::spawn(move || {
        // Remove stale FIFO if present
        let _ = fs::remove_file(&fifo);
        // Create the FIFO (named pipe)
        if let Err(e) = nix::unistd::mkfifo(fifo.as_str(), nix::sys::stat::Mode::S_IRWXU) {
            log::error!("Failed to create FIFO at {}: {}", fifo, e);
            return;
        }
        log::info!("FIFO reader listening on {}", fifo);

        let mut last_event = std::time::Instant::now()
            - std::time::Duration::from_secs(1); // ensure first event fires

        loop {
            if !running.load(Ordering::Relaxed) {
                break;
            }

            // Open FIFO (blocks until a writer connects)
            let file = match fs::File::open(&fifo) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Cannot open FIFO {}: {}", fifo, e);
                    break;
                }
            };

            use std::io::BufRead;
            let reader = io::BufReader::new(file);
            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(e) => {
                        log::error!("FIFO read error: {}", e);
                        break;
                    }
                };

                let trimmed = line.trim();
                match trimmed {
                    "capslock" => {
                        // Debounce: ignore events within 400ms of the last one,
                        // because Wayfire may fire the binding on both press AND release.
                        let now = std::time::Instant::now();
                        if now.duration_since(last_event).as_millis() < 400 {
                            log::debug!("Caps Lock event debounced");
                            continue;
                        }
                        last_event = now;

                        caps_state = !caps_state;
                        let label = if caps_state {
                            "Bloq Mayús: Activado"
                        } else {
                            "Bloq Mayús: Desactivado"
                        };
                        let icon = if caps_state {
                            "capslock-enabled-symbolic"
                        } else {
                            "capslock-disabled-symbolic"
                        };
                        let _ = app.emit("caps-lock-changed", json!({ "active": caps_state }));
                        show_osd_sync(
                            icon,
                            if caps_state { 1.0 } else { 0.0 },
                            1.0,
                            label,
                            &app,
                        );
                        log::info!("Caps Lock toggled via hotkey: {}", caps_state);
                    }
                    _ => {
                        log::warn!("Unknown FIFO message: {}", trimmed);
                    }
                }
            }
        }

        let _ = fs::remove_file(&fifo);
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

fn spawn_mic_monitor(app: AppHandle, running: Arc<AtomicBool>) {
    tokio::spawn(async move {
        let mut last_muted: Option<bool> = None;

        loop {
            if !running.load(Ordering::Relaxed) {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let muted = tokio::process::Command::new("pactl")
                .args(["get-source-mute", "@DEFAULT_SOURCE@"])
                .output()
                .await
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().contains("yes"))
                .unwrap_or(false);

            if last_muted.map(|m| m == muted).unwrap_or(false) {
                continue;
            }
            last_muted = Some(muted);

            let label = if muted {
                "Micrófono: Silenciado"
            } else {
                "Micrófono: Activado"
            };
            let icon = if muted {
                "microphone-sensitivity-muted"
            } else {
                "microphone-sensitivity-high"
            };

            let _ = app.emit("mic-mute-changed", json!({ "active": muted }));
            let _ = crate::commands::osd::show_osd_internal(
                icon,
                if muted { 1.0 } else { 0.0 },
                1.0,
                label,
                &app,
            )
            .await;
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
        log::info!("Keyboard LEDs applet starting (Wayfire hotkey + FIFO + PulseAudio)");

        // Pre-create OSD window so sync dispatch can use it
        let _ = create_osd_window(&app, "capslock-disabled-symbolic", 0.0, 1.0, "").await;

        let running = Arc::new(AtomicBool::new(true));

        let fifo = fifo_path();
        spawn_fifo_reader(fifo.clone(), app.clone(), running.clone(), false);
        register_wayfire_binding(app.clone(), fifo).await;
        spawn_mic_monitor(app, running);

        Ok(())
    }
}

pub struct KeyboardLedsApplet;
