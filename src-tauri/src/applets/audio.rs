use super::Applet;
use async_trait::async_trait;
use tauri::{AppHandle, Emitter};
use std::error::Error;
use tokio::time::Duration;
use crate::audio_native::{AudioMonitor, PwDumpMonitor};
use crate::logger::{log_info, log_error, log_debug};
use crate::structs::VolumeInfo;

pub struct AudioApplet;

#[async_trait]
impl Applet for AudioApplet {
    fn name(&self) -> &'static str {
        "audio"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        log_info("AudioApplet: starting with native monitor integration");

        // Create the best available audio monitor (PipeWire native → pw-dump → pactl)
        let monitor = AudioMonitor::new().await;
        let backend_name = monitor.backend_name();
        log_info(&format!("AudioApplet: active backend = {}", backend_name));

        // Get the state receiver for volume changes
        let mut state_rx = monitor.state_rx();
        let is_event_driven = monitor.is_event_driven();

        // Spawn the main monitoring task that watches for volume changes
        // and emits events to the frontend
        let app_clone = app.clone();
        tokio::spawn(async move {
            // Keep the monitor alive for the lifetime of this task
            let _monitor = monitor;

            monitor_volume_changes(app_clone, &mut state_rx, is_event_driven).await;
        });

        Ok(())
    }
}

/// Watches the AudioMonitor's state channel for volume changes and emits
/// `volume-changed` events to the frontend. If using pactl fallback,
/// spawns a reconnection task that tries to upgrade to event-driven mode
/// every 30 seconds.
async fn monitor_volume_changes(
    app: AppHandle,
    state_rx: &mut tokio::sync::watch::Receiver<VolumeInfo>,
    initially_event_driven: bool,
) {
    // If we started in fallback mode, spawn a reconnection task
    if !initially_event_driven {
        let app_reconnect = app.clone();
        tokio::spawn(async move {
            attempt_reconnection_loop(app_reconnect).await;
        });
    }

    // Main event loop: wait for volume changes from the monitor
    loop {
        // `changed()` waits until the value in the watch channel changes
        if state_rx.changed().await.is_err() {
            // Sender was dropped - monitor died
            log_error("AudioApplet: monitor state channel closed, stopping");
            break;
        }

        // Read the latest volume info
        let volume_info = state_rx.borrow_and_update().clone();

        log_debug(&format!(
            "AudioApplet: volume update received: {}% muted={}",
            volume_info.current, volume_info.is_muted
        ));

        // Emit to frontend - this happens immediately upon receiving the change,
        // well within the 500ms requirement since we're event-driven or polling at 2s
        if let Err(e) = app.emit("volume-changed", &volume_info) {
            log_error(&format!("AudioApplet: failed to emit volume-changed: {}", e));
        }
    }
}

/// Periodically attempts to upgrade from pactl fallback to event-driven monitoring.
/// Tries every 30 seconds. On success, runs event-driven monitoring inline.
/// If the event-driven monitor dies, falls back to the reconnection loop.
async fn attempt_reconnection_loop(app: AppHandle) {
    log_info("AudioApplet: fallback mode active, will attempt PipeWire reconnection every 30s");

    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        log_debug("AudioApplet: attempting to reconnect to PipeWire (pw-dump)...");

        match PwDumpMonitor::connect().await {
            Ok(pw_dump) => {
                log_info("AudioApplet: successfully reconnected to pw-dump, switching to event-driven mode");

                let mut state_rx = pw_dump.state_rx.clone();
                let _monitor = pw_dump;

                // Run event-driven monitoring inline. When the monitor dies
                // (channel closed), break out and retry reconnection.
                loop {
                    if state_rx.changed().await.is_err() {
                        log_error("AudioApplet: reconnected monitor channel closed, retrying reconnection...");
                        break;
                    }

                    let volume_info = state_rx.borrow_and_update().clone();

                    log_debug(&format!(
                        "AudioApplet [reconnected]: volume update: {}% muted={}",
                        volume_info.current, volume_info.is_muted
                    ));

                    if let Err(e) = app.emit("volume-changed", &volume_info) {
                        log_error(&format!("AudioApplet: failed to emit volume-changed: {}", e));
                    }
                }
                // Fall through to outer loop → retry reconnection
            }
            Err(e) => {
                log_debug(&format!(
                    "AudioApplet: PipeWire reconnection failed ({}), will retry in 30s",
                    e
                ));
            }
        }
    }
}
