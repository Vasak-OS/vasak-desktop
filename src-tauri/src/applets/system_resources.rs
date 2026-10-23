//! CPU, memoria y temperatura al panel (vasak-desktop#216).
//!
//! Son los mismos contadores que el widget de escritorio lee a pedido
//! (`system_stats`), pero acá se empujan por un evento cada dos segundos para
//! que la píldora del panel no lleve temporizador —igual que la tasa de red
//! (`network_rate`)—. La lectura es una sola para los dos: `system_stats::read`.
//!
//! Se emiten los contadores **crudos**; la interfaz saca de dos lecturas el uso
//! de CPU, con la misma cuenta que el widget (`system-stats.ts`), para no tener
//! dos copias de la fórmula.

use super::Applet;
use async_trait::async_trait;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Cada cuánto se mide, como en la tasa de red: dos segundos se sienten vivos y
/// son ocho lecturas de archivos en RAM por minuto.
const INTERVALO: Duration = Duration::from_secs(2);

pub struct SystemResourcesApplet;

#[async_trait]
impl Applet for SystemResourcesApplet {
    fn name(&self) -> &'static str {
        "system_resources"
    }

    async fn start(&self, app_handle: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            match crate::system_stats::read() {
                Ok(stats) => {
                    let _ = app_handle.emit("system-resources", stats);
                }
                // Sin /proc no hay nada que medir; la píldora simplemente no
                // aparece. No se corta el bucle: puede ser un tropiezo puntual.
                Err(error) => log::warn!("[recursos] no se pudieron leer los contadores: {error}"),
            }

            tokio::time::sleep(INTERVALO).await;
        }
    }
}
