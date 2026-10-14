use super::Applet;
use crate::audio_native::AudioMonitor;
use crate::commands::osd::show_osd_internal;
use crate::logger::{log_debug, log_error, log_info};
use crate::structs::VolumeInfo;
use async_trait::async_trait;
use std::error::Error;
use tauri::{AppHandle, Emitter};
use tokio::time::Duration;

pub struct AudioApplet;

#[async_trait]
impl Applet for AudioApplet {
    fn name(&self) -> &'static str {
        "audio"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        log_info("AudioApplet: starting with native monitor integration (lazy)");

        // Clone app for the three background tasks
        let app_for_monitor = app.clone();
        let app_for_bluetooth = app.clone();
        let _app_for_input = app.clone();

        // Defer AudioMonitor creation to a background task to avoid blocking startup
        tokio::spawn(async move {
            let monitor = AudioMonitor::new().await;
            run_audio_monitor_loop(app_for_monitor, monitor).await;
        });

        // Spawn other tasks without blocking
        tokio::spawn(async move {
            forward_bluetooth_profiles(app_for_bluetooth).await;
        });

        tokio::spawn(async move {
            forward_audio_input(app).await;
        });

        Ok(())
    }
}

/// Reenvía a la interfaz los cambios de perfil de los dispositivos Bluetooth
/// que lee `pw-dump` (`bluetooth_audio_profile`), como
/// `bluetooth-audio-profile-changed` con `{ address, profile, description,
/// codec }`. Si se atrasa y pierde avisos, sigue: el applet vuelve a pedir el
/// perfil al abrirse.
async fn forward_bluetooth_profiles(app: AppHandle) {
    use tokio::sync::broadcast::error::RecvError;

    let mut profiles = crate::bluetooth_audio_profile::subscribe();
    loop {
        match profiles.recv().await {
            Ok(profile) => {
                if let Err(e) = app.emit("bluetooth-audio-profile-changed", &profile) {
                    log_error(&format!(
                        "AudioApplet: failed to emit bluetooth-audio-profile-changed: {}",
                        e
                    ));
                }
            }
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => break,
        }
    }
}

/// Reenvía a la interfaz los cambios del micrófono que lee `pw-dump`
/// (`audio_input`, vasak-desktop#182): `microphone-changed` con el volumen y
/// el silencio de la fuente por omisión (o `null`), y
/// `audio-input-devices-changed` con las entradas. Cada uno sólo si cambió lo
/// suyo. Si se atrasa, sigue con el último: es el estado entero, no un delta.
async fn forward_audio_input(app: AppHandle) {
    use tokio::sync::broadcast::error::RecvError;

    let mut changes = crate::audio_input::subscribe();
    let mut last: Option<crate::audio_input::AudioInputSnapshot> = None;
    loop {
        let snapshot = match changes.recv().await {
            Ok(snapshot) => snapshot,
            Err(RecvError::Lagged(_)) => match crate::audio_input::snapshot() {
                Some(snapshot) => snapshot,
                None => continue,
            },
            Err(RecvError::Closed) => break,
        };
        let (microphone, devices) = audio_input_changes(last.as_ref(), &snapshot);
        if microphone {
            if let Err(e) = app.emit("microphone-changed", &snapshot.microphone) {
                log_error(&format!(
                    "AudioApplet: failed to emit microphone-changed: {}",
                    e
                ));
            }
        }
        if devices {
            if let Err(e) = app.emit("audio-input-devices-changed", &snapshot.devices) {
                log_error(&format!(
                    "AudioApplet: failed to emit audio-input-devices-changed: {}",
                    e
                ));
            }
        }
        last = Some(snapshot);
    }
}

/// Qué cambió entre lo último que se avisó y lo nuevo: (micrófono, entradas).
fn audio_input_changes(
    last: Option<&crate::audio_input::AudioInputSnapshot>,
    next: &crate::audio_input::AudioInputSnapshot,
) -> (bool, bool) {
    match last {
        None => (true, true),
        Some(last) => (
            last.microphone != next.microphone,
            last.devices != next.devices,
        ),
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

        log_info(&format!(
            "AudioApplet: active backend = {}",
            monitor.backend_name()
        ));

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
                    // El panel se mantiene al día con cada cambio del sink por
                    // omisión; el OSD, en cambio, sólo ante un cambio real del
                    // nivel o el silencio (vasak-desktop#231).
                    let show_osd = osd_warranted(last_volume.as_ref(), &volume_info);
                    last_volume = Some(volume_info.clone());

                    log_debug(&format!(
                        "AudioApplet: volume update: {}% muted={}",
                        volume_info.current, volume_info.is_muted
                    ));

                    if let Err(e) = app.emit("volume-changed", &volume_info) {
                        log_error(&format!(
                            "AudioApplet: failed to emit volume-changed: {}",
                            e
                        ));
                    }

                    if show_osd {
                        show_volume_osd(&app, &volume_info).await;
                    }
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

/// Si un cambio del sink por omisión justifica mostrar el OSD de volumen.
///
/// Sólo ante un cambio real del nivel o del silencio, y nunca en la primera
/// lectura: lo que hay al arrancar la sesión es el punto de partida, no un
/// cambio (igual que el OSD de brillo, ver `applets/brightness.rs`). El monitor
/// ya reporta sólo el sink por omisión (vasak-desktop#231); esto evita además
/// el OSD espurio al iniciar la sesión.
fn osd_warranted(last: Option<&VolumeInfo>, next: &VolumeInfo) -> bool {
    match last {
        None => false,
        Some(last) => last.current != next.current || last.is_muted != next.is_muted,
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
    let _ = show_osd_internal(
        icon,
        volume_info.current as f64,
        volume_info.max as f64,
        label,
        app,
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_input::AudioInputSnapshot;

    fn snapshot(current: i64, muted: bool, devices: usize) -> AudioInputSnapshot {
        AudioInputSnapshot {
            microphone: Some(VolumeInfo {
                current,
                min: 0,
                max: 100,
                is_muted: muted,
            }),
            devices: (0..devices)
                .map(|i| crate::structs::AudioDevice {
                    id: format!("source-{i}"),
                    name: format!("Entrada {i}"),
                    description: format!("source-{i}"),
                    is_default: i == 0,
                    volume: 0.5,
                    card: None,
                    profile: None,
                    active: true,
                })
                .collect(),
        }
    }

    #[test]
    fn la_primera_vez_se_avisan_las_dos_cosas() {
        assert_eq!(
            audio_input_changes(None, &snapshot(50, false, 1)),
            (true, true)
        );
    }

    #[test]
    fn un_cambio_de_silencio_avisa_solo_el_microfono() {
        let last = snapshot(50, false, 1);
        assert_eq!(
            audio_input_changes(Some(&last), &snapshot(50, true, 1)),
            (true, false)
        );
    }

    #[test]
    fn una_entrada_nueva_avisa_solo_la_lista() {
        let last = snapshot(50, false, 1);
        assert_eq!(
            audio_input_changes(Some(&last), &snapshot(50, false, 2)),
            (false, true)
        );
    }

    fn volume(current: i64, muted: bool) -> VolumeInfo {
        VolumeInfo {
            current,
            min: 0,
            max: 100,
            is_muted: muted,
        }
    }

    #[test]
    fn la_primera_lectura_no_muestra_el_osd() {
        // Lo que hay al arrancar la sesión es el punto de partida, no un
        // cambio: nada de OSD de «Volumen: 100 %» espurio al iniciar
        // (vasak-desktop#231).
        assert!(!osd_warranted(None, &volume(100, false)));
    }

    #[test]
    fn re_emitir_el_mismo_volumen_no_muestra_el_osd() {
        let last = volume(45, false);
        assert!(!osd_warranted(Some(&last), &volume(45, false)));
    }

    #[test]
    fn un_cambio_de_nivel_muestra_el_osd() {
        let last = volume(45, false);
        assert!(osd_warranted(Some(&last), &volume(50, false)));
    }

    #[test]
    fn un_cambio_de_silencio_muestra_el_osd() {
        let last = volume(50, false);
        assert!(osd_warranted(Some(&last), &volume(50, true)));
    }
}
