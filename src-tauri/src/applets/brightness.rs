//! El aviso de que cambió el brillo: el evento `brightness-changed` para el
//! deslizador del centro de control y el OSD.
//!
//! Antes vigilaba `/sys/class/backlight` con inotify y, si el controlador no
//! avisaba, sondeaba cada 200 ms–2 s. Ahora escucha
//! `display-brightness-changed` de `tauri-plugin-display-manager`, que lo
//! dispara el uevent del kernel: cambia el brillo —por una tecla, por otro
//! programa o por el deslizador— y llega el aviso. Sin sondeo y sin un hilo
//! propio; el del plugin ya estaba escuchando.

use std::error::Error;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tauri::{AppHandle, Emitter, Listener};
use tauri_plugin_display_manager::{BrightnessReport, DisplayManagerExt, BRIGHTNESS_EVENT};

use super::Applet;
use crate::brightness::backlight;
use crate::commands::osd::show_osd_internal;

pub struct BrightnessApplet;

#[async_trait]
impl Applet for BrightnessApplet {
    fn name(&self) -> &'static str {
        "brightness"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        // Lo que hay al arrancar es el punto de partida, no un cambio: no se
        // muestra el OSD al iniciar la sesión.
        let last = Arc::new(Mutex::new(percent_of(&app.display_manager().report())));
        let handle = app.clone();
        app.listen(BRIGHTNESS_EVENT, move |event| {
            let Ok(report) = serde_json::from_str::<BrightnessReport>(event.payload()) else {
                return;
            };
            let Some(percent) = last
                .lock()
                .ok()
                .and_then(|mut last| changed(&mut last, &report))
            else {
                return;
            };
            announce(&handle, percent);
        });
        log::info!("Brightness applet escuchando {BRIGHTNESS_EVENT}");
        Ok(())
    }
}

fn percent_of(report: &BrightnessReport) -> Option<u8> {
    backlight(report).map(|monitor| monitor.percent)
}

/// El brillo nuevo del panel, si cambió. El evento del plugin llega también
/// cuando cambian los monitores o el brillo de uno externo; eso no mueve el
/// deslizador ni muestra el OSD.
pub fn changed(last: &mut Option<u8>, report: &BrightnessReport) -> Option<u8> {
    let percent = percent_of(report)?;
    if *last == Some(percent) {
        return None;
    }
    *last = Some(percent);
    Some(percent)
}

fn announce(app: &AppHandle, percent: u8) {
    let _ = app.emit(
        "brightness-changed",
        serde_json::json!({
            "current": percent,
            "max": 100,
            "min": 0
        }),
    );
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = show_osd_internal(
            "display-brightness",
            f64::from(percent),
            100.0,
            "osd.brightness",
            &app,
        )
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri_plugin_display_manager::{BrightnessKind, DdcState, DdcStatus, MonitorBrightness};

    fn report(monitors: Vec<(BrightnessKind, u8)>) -> BrightnessReport {
        BrightnessReport {
            monitors: monitors
                .into_iter()
                .map(|(kind, percent)| MonitorBrightness {
                    output: None,
                    kind,
                    handle: "x".into(),
                    percent,
                })
                .collect(),
            ddc: DdcStatus {
                state: DdcState::Ready,
                reason: None,
                unsupported: vec![],
            },
        }
    }

    #[test]
    fn solo_avisa_cuando_cambia_el_panel() {
        let mut last = Some(40);
        assert_eq!(
            changed(&mut last, &report(vec![(BrightnessKind::Backlight, 40)])),
            None
        );
        assert_eq!(
            changed(&mut last, &report(vec![(BrightnessKind::Backlight, 55)])),
            Some(55)
        );
        assert_eq!(last, Some(55));
        assert_eq!(
            changed(
                &mut last,
                &report(vec![
                    (BrightnessKind::Ddc, 10),
                    (BrightnessKind::Backlight, 55)
                ])
            ),
            None,
            "cambió un monitor externo, no el panel"
        );
    }

    #[test]
    fn sin_panel_no_avisa_nada() {
        let mut last = None;
        assert_eq!(
            changed(&mut last, &report(vec![(BrightnessKind::Ddc, 10)])),
            None
        );
        assert_eq!(last, None);
    }

    /// El evento que emite el plugin se entiende tal cual llega.
    #[test]
    fn lee_el_evento_del_plugin() {
        let json = r#"{"monitors":[{"output":"eDP-1","kind":"backlight","handle":"intel_backlight","percent":70}],
                       "ddc":{"state":"ready","reason":null,"unsupported":[]}}"#;
        let parsed: BrightnessReport = serde_json::from_str(json).unwrap();
        let mut last = None;
        assert_eq!(changed(&mut last, &parsed), Some(70));
    }
}
