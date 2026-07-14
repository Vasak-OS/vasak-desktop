use super::Applet;
use async_trait::async_trait;
use tauri::{AppHandle, Emitter};
use std::error::Error;
use tokio::time::Duration;
use crate::audio_native::{AudioMonitor, PwDumpMonitor};
use crate::commands::osd::show_osd_internal;
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

        let monitor = AudioMonitor::new().await;
        let backend_name = monitor.backend_name();
        log_info(&format!("AudioApplet: active backend = {}", backend_name));

        let mut state_rx = monitor.state_rx();
        let is_event_driven = monitor.is_event_driven();

        let app_clone = app.clone();
        tokio::spawn(async move {
            let _monitor = monitor;
            monitor_volume_changes(app_clone, &mut state_rx, is_event_driven).await;
        });

        Ok(())
    }
}

async fn monitor_volume_changes(
    app: AppHandle,
    state_rx: &mut tokio::sync::watch::Receiver<VolumeInfo>,
    initially_event_driven: bool,
) {
    if !initially_event_driven {
        let app_reconnect = app.clone();
        tokio::spawn(async move {
            attempt_reconnection_loop(app_reconnect).await;
        });
    }

    loop {
        if state_rx.changed().await.is_err() {
            log_error("AudioApplet: monitor state channel closed, stopping");
            break;
        }

        let volume_info = state_rx.borrow_and_update().clone();

        log_debug(&format!(
            "AudioApplet: volume update received: {}% muted={}",
            volume_info.current, volume_info.is_muted
        ));

        if let Err(e) = app.emit("volume-changed", &volume_info) {
            log_error(&format!("AudioApplet: failed to emit volume-changed: {}", e));
        }

        show_volume_osd(&app, &volume_info).await;
    }
}

fn get_volume_icon_name(is_muted: bool, percentage: u8) -> &'static str {
    if is_muted {
        "audio-volume-muted"
    } else if percentage > 66 {
        "audio-volume-high"
    } else if percentage > 33 {
        "audio-volume-medium"
    } else {
        "audio-volume-low"
    }
}

fn get_volume_percentage(current: i64, min: i64, max: i64) -> u8 {
    if max > min {
        ((current - min) * 100 / (max - min)) as u8
    } else {
        0
    }
}

async fn show_volume_osd(app: &AppHandle, volume_info: &VolumeInfo) {
    let percentage = get_volume_percentage(volume_info.current, volume_info.min, volume_info.max);
    let icon = get_volume_icon_name(volume_info.is_muted, percentage);
    let label = if volume_info.is_muted {
        "Silenciado".to_string()
    } else {
        format!("Volumen: {}%", percentage)
    };
    let _ = show_osd_internal(icon, volume_info.current as f64, volume_info.max as f64, &label, app).await;
}

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

                    show_volume_osd(&app, &volume_info).await;
                }
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
