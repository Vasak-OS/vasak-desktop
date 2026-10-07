//! El OSD de brillo.
//!
//! Escucha `display-brightness-changed` de `tauri-plugin-display-manager`, que
//! lo dispara el uevent del kernel: cambia el brillo del panel —por una tecla,
//! por otro programa o por el deslizador— y aparece el OSD. Sin sondeo y sin un
//! hilo propio; el del plugin ya estaba escuchando.
//!
//! Los deslizadores del centro de control escuchan el mismo evento del plugin
//! directamente (vasak-desktop#189): acá no se reenvía nada al frontend.

use std::error::Error;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tauri::{AppHandle, Listener};
use tauri_plugin_display_manager::{
    BrightnessKind, BrightnessReport, DisplayManagerExt, MonitorBrightness, BRIGHTNESS_EVENT,
};

use super::Applet;
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

/// El panel interno: el que regulan las teclas de brillo y el que anuncia el
/// OSD. El plugin ya elige cuál es cuando hay más de una retroiluminación (la
/// que cuelga de un conector interno).
pub fn backlight(report: &BrightnessReport) -> Option<&MonitorBrightness> {
    report
        .monitors
        .iter()
        .find(|monitor| monitor.kind == BrightnessKind::Backlight)
}

fn percent_of(report: &BrightnessReport) -> Option<u8> {
    backlight(report).map(|monitor| monitor.percent)
}

/// El brillo nuevo del panel, si cambió. El evento del plugin llega también
/// cuando cambian los monitores o el brillo de uno externo; eso no muestra el
/// OSD.
pub fn changed(last: &mut Option<u8>, report: &BrightnessReport) -> Option<u8> {
    let percent = percent_of(report)?;
    if *last == Some(percent) {
        return None;
    }
    *last = Some(percent);
    Some(percent)
}

fn announce(app: &AppHandle, percent: u8) {
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
    use tauri_plugin_display_manager::{DdcState, DdcStatus};

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

    #[test]
    fn el_osd_es_el_del_panel_interno() {
        let both = BrightnessReport {
            monitors: vec![
                MonitorBrightness {
                    output: Some("HDMI-A-1".into()),
                    kind: BrightnessKind::Ddc,
                    handle: "5".into(),
                    percent: 80,
                },
                MonitorBrightness {
                    output: Some("eDP-1".into()),
                    kind: BrightnessKind::Backlight,
                    handle: "intel_backlight".into(),
                    percent: 40,
                },
            ],
            ddc: DdcStatus {
                state: DdcState::Ready,
                reason: None,
                unsupported: vec![],
            },
        };
        assert_eq!(backlight(&both).unwrap().handle, "intel_backlight");
    }
}
