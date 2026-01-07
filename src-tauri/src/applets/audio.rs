use super::Applet;
use async_trait::async_trait;
use tauri::{AppHandle, Emitter};
use std::error::Error;
use tokio::time::Duration;
use crate::constants::{AUDIO_SLOW_POLL_MS, AUDIO_FAST_POLL_MS, AUDIO_FAST_POLL_ITERATIONS};

pub struct AudioApplet;

#[async_trait]
impl Applet for AudioApplet {
    fn name(&self) -> &'static str {
        "audio"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        log::info!("Starting audio monitoring");
        
        // Spawn monitoring task
        tokio::spawn(async move {
            monitor_audio_changes(app).await;
        });
        
        Ok(())
    }
}

/// Monitorea cambios en el volumen de audio con polling adaptativo
async fn monitor_audio_changes(app: AppHandle) {
    let mut current_interval_ms = AUDIO_FAST_POLL_MS; // Start fast
    let mut fast_poll_countdown = AUDIO_FAST_POLL_ITERATIONS;
    let mut last_volume: Option<crate::structs::VolumeInfo> = None;

    loop {
        tokio::time::sleep(Duration::from_millis(current_interval_ms)).await;

        // Obtener volumen actual
        match crate::audio::get_volume() {
            Ok(current_volume) => {
                // Comparar con el último valor conocido
                let has_changed = match &last_volume {
                    None => true, // Primera lectura
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

                    // Emitir evento al frontend
                    if let Err(e) = app.emit("volume-changed", &current_volume) {
                        log::error!("Failed to emit volume-changed event: {}", e);
                    }

                    last_volume = Some(current_volume);
                    
                    // Switch to fast polling on change
                    current_interval_ms = AUDIO_FAST_POLL_MS;
                    fast_poll_countdown = AUDIO_FAST_POLL_ITERATIONS;
                } else {
                    // No change detected
                    if fast_poll_countdown > 0 {
                        fast_poll_countdown -= 1;
                    } else {
                        // Switch to slow polling if stable
                        current_interval_ms = AUDIO_SLOW_POLL_MS;
                    }
                }
            },
            Err(_) => {
                // Si falla la lectura, usar intervalo lento
                current_interval_ms = AUDIO_SLOW_POLL_MS;
            }
        }
    }
}
