use super::Applet;
use async_trait::async_trait;
use tauri::{AppHandle, Emitter};
use std::error::Error;
use tokio::time::Duration;
use crate::audio_native::AudioMonitor;
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

        tokio::spawn(async move {
            run_audio_monitor_loop(app, monitor).await;
        });

        Ok(())
    }
}

/// Single-owner audio monitor loop. One task, one active monitor at a time.
/// When the monitor's channel closes (backend failed), the old monitor is
/// dropped and a fresh one is created — no duplicate backends, no separate
/// reconnection task.
async fn run_audio_monitor_loop(app: AppHandle, mut monitor: AudioMonitor) {
    'outer: loop {
        let is_event_driven = monitor.is_event_driven();
        let mut state_rx = monitor.state_rx();

        log_info(&format!("AudioApplet: active backend = {}", monitor.backend_name()));

        // Track last emitted state to avoid redundant JS events
        let mut last_volume: Option<VolumeInfo> = None;

        // Polling-fallback upgrade timer: periodically try event-driven backends.
        let mut upgrade_timer = tokio::time::interval(Duration::from_secs(30));

        // Inner monitor loop — awaits state changes.
        loop {
            let result = if is_event_driven {
                state_rx.changed().await
            } else {
                tokio::select! {
                    biased;
                    result = state_rx.changed() => result,
                    _ = upgrade_timer.tick() => {
                        // Polling fallback: try to upgrade to event-driven.
                        let new_monitor = AudioMonitor::new().await;
                        if new_monitor.is_event_driven() {
                            log_info("AudioApplet: upgrading from polling to event-driven backend");
                            drop(monitor);
                            monitor = new_monitor;
                            continue 'outer; // skip reconnect, re-enter outer loop with upgraded monitor
                        }
                        // Still only polling available — keep the old one.
                        continue;
                    }
                }
            };

            match result {
                Ok(()) => {
                    let volume_info = state_rx.borrow_and_update().clone();

                    // Only emit if the state actually changed
                    if last_volume.as_ref() == Some(&volume_info) {
                        continue;
                    }
                    last_volume = Some(volume_info.clone());

                    log_debug(&format!(
                        "AudioApplet: volume update: {}% muted={}",
                        volume_info.current, volume_info.is_muted
                    ));

                    if let Err(e) = app.emit("volume-changed", &volume_info) {
                        log_error(&format!("AudioApplet: failed to emit volume-changed: {}", e));
                    }

                    show_volume_osd(&app, &volume_info).await;
                }
                Err(_) => {
                    log_error("AudioApplet: monitor channel closed — reconnecting");
                    break;
                }
            }
        }

        // Monitor is dead. Drop it and create a replacement.
        drop(monitor);
        monitor = AudioMonitor::new().await;
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
    // A locale key: the OSD view translates it and fills in the percentage,
    // which it can derive from the value and maximum it already receives.
    let label = if volume_info.is_muted {
        "osd.muted"
    } else {
        "osd.volume"
    };
    let _ = show_osd_internal(icon, volume_info.current as f64, volume_info.max as f64, label, app).await;
}
