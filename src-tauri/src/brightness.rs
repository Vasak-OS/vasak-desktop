//! El brillo del panel interno, por `tauri-plugin-display-manager`
//! (vasak-desktop#178).
//!
//! Antes se detectaba un método entre `brightnessctl`, `busctl` y sysfs, y
//! cada lectura y escritura era un subproceso. Ahora es el plugin que comparte
//! con vasak-settings: lee sysfs en el momento (dos archivos) y escribe por el
//! `SetBrightness` de logind en el D-Bus del sistema, sin subprocesos ni
//! permisos especiales.
//!
//! El centro de control muestra **un** deslizador, el del panel interno, como
//! antes. Los monitores externos (DDC/CI) los ve Configuración; desde acá no se
//! les habla, así que `ddcutil` nunca corre por el escritorio.

use tauri::{AppHandle, Runtime};
use tauri_plugin_display_manager::{
    BrightnessKind, BrightnessReport, DisplayManagerExt, MonitorBrightness,
};

use crate::error::{Result, VasakError};
use crate::logger::{log_debug, log_error};
use crate::structs::BrightnessInfo;

/// La pantalla que regula el deslizador del centro: la primera con
/// retroiluminación. El plugin ya elige cuál es la del panel cuando hay más de
/// una (la que cuelga de un conector interno).
pub fn backlight(report: &BrightnessReport) -> Option<&MonitorBrightness> {
    report
        .monitors
        .iter()
        .find(|monitor| monitor.kind == BrightnessKind::Backlight)
}

/// El brillo del panel en la forma que espera el frontend.
pub fn info(report: &BrightnessReport) -> Result<BrightnessInfo> {
    backlight(report)
        .map(|monitor| BrightnessInfo {
            current: u32::from(monitor.percent),
            max: 100,
            min: 0,
        })
        .ok_or_else(|| VasakError::NotFound("No backlight devices found".to_string()))
}

/// Lo que se sabe ahora, sin hablarle a ningún monitor externo: `report` y no
/// `get`, que buscaría los externos por DDC/CI en segundo plano.
pub fn get_brightness<R: Runtime>(app: &AppHandle<R>) -> Result<BrightnessInfo> {
    let result = info(&app.display_manager().report());
    if let Ok(ref info) = result {
        log_debug(&format!("Brillo actual: {}%", info.current));
    }
    result
}

/// Pone el brillo del panel. El aviso del cambio llega solo: la escritura
/// dispara un uevent del kernel, y el plugin lo emite como
/// `display-brightness-changed`.
pub async fn set_brightness<R: Runtime>(app: &AppHandle<R>, percent: u32) -> Result<()> {
    let manager = app.display_manager();
    let handle = backlight(&manager.report())
        .map(|monitor| monitor.handle.clone())
        .ok_or_else(|| VasakError::NotFound("No backlight devices found".to_string()))?;
    let percent = percent.min(100) as u8;
    manager
        .set(BrightnessKind::Backlight, &handle, percent)
        .await
        .map_err(|e| {
            log_error(&format!("Error al establecer brillo: {e}"));
            VasakError::Brightness(e.to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri_plugin_display_manager::{DdcState, DdcStatus};

    fn report(monitors: Vec<MonitorBrightness>) -> BrightnessReport {
        BrightnessReport {
            monitors,
            ddc: DdcStatus {
                state: DdcState::Ready,
                reason: None,
                unsupported: vec![],
            },
        }
    }

    fn screen(kind: BrightnessKind, handle: &str, percent: u8) -> MonitorBrightness {
        MonitorBrightness {
            output: None,
            kind,
            handle: handle.into(),
            percent,
        }
    }

    #[test]
    fn el_deslizador_es_el_del_panel_interno() {
        let both = report(vec![
            screen(BrightnessKind::Ddc, "5", 80),
            screen(BrightnessKind::Backlight, "intel_backlight", 40),
        ]);
        assert_eq!(backlight(&both).unwrap().handle, "intel_backlight");
        assert_eq!(
            info(&both).unwrap(),
            BrightnessInfo {
                current: 40,
                max: 100,
                min: 0
            }
        );
    }

    #[test]
    fn sin_panel_interno_no_hay_brillo() {
        let external = report(vec![screen(BrightnessKind::Ddc, "5", 80)]);
        assert!(backlight(&external).is_none());
        assert!(info(&external).is_err());
    }
}
