use super::Applet;
use async_trait::async_trait;
use tauri::{AppHandle, Emitter};
use std::error::Error;
use tokio::time::Duration;
use crate::commands::osd::show_osd_internal;
use crate::constants::{AUDIO_SLOW_POLL_MS, AUDIO_FAST_POLL_MS, AUDIO_FAST_POLL_ITERATIONS};

pub struct AudioApplet;

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

#[async_trait]
impl Applet for AudioApplet {
    fn name(&self) -> &'static str {
        "audio"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        log::info!("Starting audio monitoring");

        tokio::spawn(async move {
            monitor_audio_changes(app).await;
        });

        Ok(())
    }
}

async fn monitor_audio_changes(app: AppHandle) {
    let mut current_interval_ms = AUDIO_FAST_POLL_MS;
    let mut fast_poll_countdown = AUDIO_FAST_POLL_ITERATIONS;
    let mut last_volume: Option<crate::structs::VolumeInfo> = None;

    loop {
        tokio::time::sleep(Duration::from_millis(current_interval_ms)).await;

        match crate::audio::get_volume() {
            Ok(current_volume) => {
                let has_changed = match &last_volume {
                    None => true,
                    Some(last) => {
                        last.current != current_volume.current || last.is_muted != current_volume.is_muted
                    }
                };

                if has_changed {
                    log::debug!(
                        "Audio changed: volume={}%, muted={}",
                        current_volume.current,
                        current_volume.is_muted
                    );

                    if let Err(e) = app.emit("volume-changed", &current_volume) {
                        log::error!("Failed to emit volume-changed event: {}", e);
                    }

                    let percentage = get_volume_percentage(
                        current_volume.current,
                        current_volume.min,
                        current_volume.max,
                    );
                    let icon = get_volume_icon_name(current_volume.is_muted, percentage);
                    let label = if current_volume.is_muted {
                        "Silenciado".to_string()
                    } else {
                        format!("Volumen: {}%", percentage)
                    };

                    let _ = show_osd_internal(
                        icon,
                        current_volume.current as f64,
                        current_volume.max as f64,
                        &label,
                        &app,
                    )
                    .await;

                    last_volume = Some(current_volume);

                    current_interval_ms = AUDIO_FAST_POLL_MS;
                    fast_poll_countdown = AUDIO_FAST_POLL_ITERATIONS;
                } else {
                    if fast_poll_countdown > 0 {
                        fast_poll_countdown -= 1;
                    } else {
                        current_interval_ms = AUDIO_SLOW_POLL_MS;
                    }
                }
            },
            Err(_) => {
                current_interval_ms = AUDIO_SLOW_POLL_MS;
            }
        }
    }
}
