use super::Applet;
use async_trait::async_trait;
use tauri::{AppHandle, Emitter};
use std::error::Error;
use tokio::time::{interval, Duration};

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

/// Monitorea cambios en el volumen de audio
async fn monitor_audio_changes(app: AppHandle) {
    let mut interval = interval(Duration::from_millis(500)); // Poll cada 500ms
    let mut last_volume: Option<crate::structs::VolumeInfo> = None;

    loop {
        interval.tick().await;

        // Obtener volumen actual
        if let Ok(current_volume) = crate::audio::get_volume() {
            // Comparar con el último valor conocido
            let should_emit = match &last_volume {
                None => true, // Primera lectura
                Some(last) => {
                    // Emitir si cambió el volumen o el estado de mute
                    last.current != current_volume.current || last.is_muted != current_volume.is_muted
                }
            };

            if should_emit {
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
            }
        } else {
            // Si falla la lectura, esperar más tiempo antes del próximo intento
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}
