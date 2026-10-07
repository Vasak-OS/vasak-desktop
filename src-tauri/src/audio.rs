use crate::constants::CMD_PACTL;
use crate::error::{Result, VasakError};
use crate::logger::{log_debug, log_error, log_info};
use crate::structs::{AudioDevice, VolumeInfo};
use crate::utils::CommandExecutor;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

fn sink_cache() -> &'static Mutex<Option<(String, Instant)>> {
    static CACHE: OnceLock<Mutex<Option<(String, Instant)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn clear_sink_cache() {
    if let Ok(mut cache) = sink_cache().lock() {
        cache.take();
    }
}

/// Obtiene el nombre del sink de audio por defecto (con caché de 2s)
fn get_default_sink_name() -> Result<String> {
    if let Ok(cache) = sink_cache().lock() {
        if let Some((name, time)) = cache.as_ref() {
            if time.elapsed() < std::time::Duration::from_secs(2) {
                return Ok(name.clone());
            }
        }
    }

    let info_output = CommandExecutor::run(CMD_PACTL, &["info"])?;

    let default_sink = info_output
        .lines()
        .find_map(|line| {
            // `pactl` cambió la capitalización de esta etiqueta entre versiones.
            let trimmed = line.trim();
            trimmed
                .strip_prefix("Default Sink:")
                .or_else(|| trimmed.strip_prefix("default sink:"))
                .map(|suffix| suffix.trim().to_string())
        })
        .ok_or_else(|| {
            log_error("No se encontró el sink por defecto en pactl info");
            VasakError::NotFound("No se encontró el sink por defecto".to_string())
        })?;

    if let Ok(mut cache) = sink_cache().lock() {
        let _ = cache.insert((default_sink.clone(), Instant::now()));
    }

    Ok(default_sink)
}

fn parse_volume_percent(output: &str) -> Result<i64> {
    // Busca el primer porcentaje en el texto (ej: "75%")
    output
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("Volume:") || trimmed.starts_with("volume:") {
                trimmed.split_whitespace().find_map(|part| {
                    if let Some(pct) = part.strip_suffix('%') {
                        pct.parse::<i64>().ok()
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
        .ok_or_else(|| VasakError::Parse("No se pudo parsear el porcentaje de volumen".to_string()))
}

/// Obtiene la información actual del volumen del sistema
pub fn get_volume() -> Result<VolumeInfo> {
    let sink = get_default_sink_name()?;
    let list_output = CommandExecutor::run(CMD_PACTL, &["list", "sinks"])?;

    // Encontrar la sección del sink por defecto
    let mut in_sink = false;
    let mut volume_pct = None;
    let mut is_muted = false;

    for line in list_output.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("Sink #") {
            in_sink = true;
            continue;
        }

        if !in_sink {
            continue;
        }

        // Llegamos a otro sink o al final
        if trimmed.starts_with("Sink #") || (trimmed.is_empty() && in_sink) {
            break;
        }

        if trimmed.starts_with("Name:") {
            let name = trimmed.strip_prefix("Name:").unwrap_or("").trim();
            if name != sink {
                in_sink = false;
            }
            continue;
        }

        if trimmed.starts_with("Mute:") {
            is_muted = trimmed.contains("yes");
            continue;
        }

        // Volume: front-left: 49152 /  75% / -6.70 dB, front-right: ...
        if trimmed.starts_with("Volume:") || trimmed.starts_with("volume:") {
            if let Some(pct) = trimmed
                .split_whitespace()
                .find_map(|part| part.strip_suffix('%').and_then(|s| s.parse::<i64>().ok()))
            {
                volume_pct = Some(pct);
            }
        }
    }

    // Si no encontramos datos del sink por defecto, intentar parse global
    let current = volume_pct
        .or_else(|| parse_volume_percent(&list_output).ok())
        .unwrap_or(0);

    Ok(VolumeInfo {
        current,
        min: 0,
        max: 100,
        is_muted,
    })
}

/// Establece el volumen del sistema
pub fn set_volume(volume: i64, _app: AppHandle) -> Result<()> {
    log_info(&format!("Estableciendo volumen a: {}%", volume));
    let sink = get_default_sink_name()?;
    let volume_str = format!("{}%", volume);

    CommandExecutor::run(CMD_PACTL, &["set-sink-volume", &sink, &volume_str])?;

    Ok(())
}

/// Alterna el estado de silencio del audio
pub fn toggle_mute(_app: AppHandle) -> Result<bool> {
    log_info("Alternando estado de mute");
    let sink = get_default_sink_name()?;

    CommandExecutor::run(CMD_PACTL, &["set-sink-mute", &sink, "toggle"])?;

    let info = get_volume()?;
    Ok(info.is_muted)
}

/// Lista todos los dispositivos de salida de audio (sinks)
pub fn list_audio_devices() -> Result<Vec<AudioDevice>> {
    log_debug("Listando dispositivos de audio");
    let output = CommandExecutor::run(CMD_PACTL, &["list", "sinks"])?;
    let default_sink = get_default_sink_name().ok();
    let devices = parse_sinks(&output, default_sink.as_deref());
    log_debug(&format!(
        "Encontrados {} dispositivos de audio",
        devices.len()
    ));
    Ok(devices)
}

/// El nombre del sink del ecualizador de sistema (vasak-wireplumber-modules).
const EQUALIZER_SINK: &str = "vasak-equalizer";

/// Un sink a medio leer de la salida de `pactl list sinks`.
#[derive(Default)]
struct SinkDraft {
    id: String,
    name: String,
    description: String,
    volume: Option<f64>,
    /// `filter.smart = "true"`: un filtro que WirePlumber pone delante de la
    /// salida, como el ecualizador. No es una salida que se pueda elegir.
    smart_filter: bool,
    /// `device.class = "monitor"`: el monitor de una salida, que `pactl`
    /// lista entre las fuentes. No es un micrófono.
    monitor: bool,
}

impl SinkDraft {
    fn finish(self, default_sink: Option<&str>) -> Option<AudioDevice> {
        if self.id.is_empty() || self.smart_filter || self.monitor || self.name == EQUALIZER_SINK
        {
            return None;
        }
        let name = if self.description.is_empty() {
            self.name.clone()
        } else {
            self.description
        };
        Some(AudioDevice {
            id: self.id,
            is_default: default_sink == Some(self.name.as_str()),
            name,
            description: self.name,
            volume: self.volume.unwrap_or(0.5),
        })
    }
}

/// Las salidas que se pueden elegir, a partir de `pactl list sinks`.
///
/// Saca los filtros: el ecualizador aparece como «VasakOS Equalizer» y elegirlo
/// como salida por omisión no tiene sentido —el sonido ya pasa por él—. Lo que
/// identifica a cualquier filtro, éste o uno futuro, es `filter.smart`; el
/// nombre va de respaldo, por si la propiedad no llega.
fn parse_sinks(output: &str, default_sink: Option<&str>) -> Vec<AudioDevice> {
    parse_devices(output, "Sink #", default_sink)
}

/// Las entradas que se pueden elegir, a partir de `pactl list sources`.
///
/// Es el respaldo cuando no hay flujo de `pw-dump` (ver `audio_input`). Saca
/// los monitores de las salidas, que `pactl` lista como fuentes, y usa el
/// nombre del nodo como identificador, igual que el flujo.
fn parse_sources(output: &str, default_source: Option<&str>) -> Vec<AudioDevice> {
    parse_devices(output, "Source #", default_source)
        .into_iter()
        .map(|device| AudioDevice {
            id: device.description.clone(),
            ..device
        })
        .collect()
}

/// Los dispositivos de `pactl list sinks` o `pactl list sources`, según el
/// encabezado de cada bloque.
fn parse_devices(output: &str, header: &str, default_name: Option<&str>) -> Vec<AudioDevice> {
    let mut devices = Vec::new();
    let mut current: Option<SinkDraft> = None;

    for line in output.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix(header) {
            if let Some(device) = current.take().and_then(|draft| draft.finish(default_name)) {
                devices.push(device);
            }
            current = Some(SinkDraft {
                id: rest.split_whitespace().next().unwrap_or("").to_string(),
                ..SinkDraft::default()
            });
            continue;
        }

        let Some(draft) = current.as_mut() else {
            continue;
        };

        if let Some(rest) = trimmed.strip_prefix("Name:") {
            draft.name = rest.trim().to_string();
        } else if let Some(rest) = trimmed.strip_prefix("Description:") {
            draft.description = rest.trim().to_string();
        } else if trimmed.starts_with("Volume:") || trimmed.starts_with("volume:") {
            if let Some(pct) = trimmed
                .split_whitespace()
                .find_map(|part| part.strip_suffix('%').and_then(|s| s.parse::<f64>().ok()))
            {
                draft.volume = Some(pct / 100.0);
            }
        } else if let Some(value) = trimmed.strip_prefix("filter.smart") {
            draft.smart_filter = value.trim_start().trim_start_matches('=').trim() == "\"true\"";
        } else if let Some(value) = trimmed.strip_prefix("device.class") {
            draft.monitor = value.trim_start().trim_start_matches('=').trim() == "\"monitor\"";
        } else if trimmed.is_empty() && !draft.name.is_empty() {
            // Fin de este sink: la línea vacía que separa uno del siguiente.
            if let Some(device) = current.take().and_then(|draft| draft.finish(default_name)) {
                devices.push(device);
            }
        }
    }

    if let Some(device) = current.take().and_then(|draft| draft.finish(default_name)) {
        devices.push(device);
    }
    devices
}

/// Establece el dispositivo de salida de audio por defecto
pub fn set_default_audio_device(device_id: &str, app: AppHandle) -> Result<()> {
    log_info(&format!(
        "Estableciendo dispositivo de audio por defecto: {}",
        device_id
    ));
    CommandExecutor::run(CMD_PACTL, &["set-default-sink", device_id])?;

    clear_sink_cache();

    if let Ok(devices) = list_audio_devices() {
        log_debug("Notificando cambio de dispositivos de audio al frontend");
        let _ = app.emit("audio-devices-changed", devices);
    }

    log_info("Dispositivo de audio por defecto establecido correctamente");
    Ok(())
}

/// La fuente por omisión, para `pactl`.
const DEFAULT_SOURCE: &str = "@DEFAULT_SOURCE@";

/// `Mute: yes` o `Mute: no`, como lo imprime `pactl get-source-mute`.
fn parse_mute(output: &str) -> bool {
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix("Mute:"))
        .is_some_and(|value| value.trim() == "yes")
}

/// El volumen y el silencio del micrófono (la fuente por omisión), o `None`
/// si no hay ninguna.
///
/// Sale del flujo de `pw-dump` que el escritorio ya tiene abierto
/// (`audio_input`); sólo sin él se pregunta a `pactl`.
pub fn get_microphone() -> Result<Option<VolumeInfo>> {
    if let Some(snapshot) = crate::audio_input::snapshot() {
        return Ok(snapshot.microphone);
    }
    // Sin fuente, `pactl` falla: no es un error, es que no hay micrófono.
    let Ok(volume) = CommandExecutor::run(CMD_PACTL, &["get-source-volume", DEFAULT_SOURCE]) else {
        return Ok(None);
    };
    let mute = CommandExecutor::run(CMD_PACTL, &["get-source-mute", DEFAULT_SOURCE])?;
    Ok(Some(VolumeInfo {
        current: parse_volume_percent(&volume)?,
        min: 0,
        max: 100,
        is_muted: parse_mute(&mute),
    }))
}

/// El volumen del micrófono, en porcentaje.
pub fn set_microphone_volume(volume: i64) -> Result<()> {
    let volume = format!("{}%", volume.clamp(0, 150));
    CommandExecutor::run(CMD_PACTL, &["set-source-volume", DEFAULT_SOURCE, &volume])?;
    Ok(())
}

/// Silencia o deja oír el micrófono; devuelve si quedó silenciado.
pub fn toggle_microphone_mute() -> Result<bool> {
    log_info("Alternando el silencio del micrófono");
    CommandExecutor::run(CMD_PACTL, &["set-source-mute", DEFAULT_SOURCE, "toggle"])?;
    let mute = CommandExecutor::run(CMD_PACTL, &["get-source-mute", DEFAULT_SOURCE])?;
    Ok(parse_mute(&mute))
}

/// Las entradas de audio que se pueden elegir.
pub fn list_audio_input_devices() -> Result<Vec<AudioDevice>> {
    if let Some(snapshot) = crate::audio_input::snapshot() {
        return Ok(snapshot.devices);
    }
    let output = CommandExecutor::run(CMD_PACTL, &["list", "sources"])?;
    let default_source = CommandExecutor::run(CMD_PACTL, &["get-default-source"]).ok();
    Ok(parse_sources(&output, default_source.as_deref().map(str::trim)))
}

/// Elige la entrada por omisión, por el nombre del nodo.
///
/// Con el flujo de `pw-dump` el cambio vuelve solo, como
/// `audio-input-devices-changed` desde el applet de audio; sin él se avisa
/// acá, para que el selector no quede viejo.
pub fn set_default_audio_input_device(device_id: &str, app: AppHandle) -> Result<()> {
    log_info(&format!("Estableciendo la entrada de audio por defecto: {}", device_id));
    CommandExecutor::run(CMD_PACTL, &["set-default-source", device_id])?;
    if crate::audio_input::snapshot().is_none() {
        if let Ok(devices) = list_audio_input_devices() {
            let _ = app.emit("audio-input-devices-changed", devices);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lo que imprime `pactl list sinks` con la salida de la placa y el
    /// ecualizador de sistema delante (recortado a lo que se lee).
    const SINKS: &str = r#"Sink #52
	State: RUNNING
	Name: alsa_output.pci-0000_00_1f.3.analog-stereo
	Description: Audio interno Estéreo analógico
	Mute: no
	Volume: front-left: 39321 /  60% / -13,31 dB,   front-right: 39321 /  60% / -13,31 dB
	Properties:
		device.class = "sound"
		media.class = "Audio/Sink"

Sink #77
	State: RUNNING
	Name: vasak-equalizer
	Description: VasakOS Equalizer
	Volume: front-left: 65536 / 100% / 0,00 dB,   front-right: 65536 / 100% / 0,00 dB
	Properties:
		node.name = "vasak-equalizer"
		filter.smart = "true"

Sink #80
	State: SUSPENDED
	Name: bluez_output.00_11_22.1
	Description: Auriculares WH-1000XM4
	Volume: front-left: 22938 /  35% / -27,36 dB,   front-right: 22938 /  35% / -27,36 dB
	Properties:
		filter.smart = "false"
"#;

    #[test]
    fn el_ecualizador_no_es_una_salida_que_se_pueda_elegir() {
        let devices = parse_sinks(SINKS, Some("alsa_output.pci-0000_00_1f.3.analog-stereo"));
        let names: Vec<_> = devices.iter().map(|d| d.description.as_str()).collect();
        assert_eq!(
            names,
            [
                "alsa_output.pci-0000_00_1f.3.analog-stereo",
                "bluez_output.00_11_22.1"
            ]
        );
    }

    #[test]
    fn las_salidas_siguen_con_su_nombre_volumen_y_la_elegida() {
        let devices = parse_sinks(SINKS, Some("alsa_output.pci-0000_00_1f.3.analog-stereo"));
        assert_eq!(devices[0].id, "52");
        assert_eq!(devices[0].name, "Audio interno Estéreo analógico");
        assert!(devices[0].is_default);
        assert!((devices[0].volume - 0.6).abs() < 1e-9);
        assert_eq!(devices[1].name, "Auriculares WH-1000XM4");
        assert!(!devices[1].is_default);
        assert!((devices[1].volume - 0.35).abs() < 1e-9);
    }

    #[test]
    fn un_filtro_inteligente_se_saca_aunque_se_llame_de_otra_forma() {
        let output = "Sink #1\n\tName: otro-filtro\n\tDescription: Filtro\n\tProperties:\n\t\tfilter.smart = \"true\"\n";
        assert!(parse_sinks(output, None).is_empty());
    }

    /// Lo que imprime `pactl list sources`: el micrófono y el monitor de la
    /// salida, que no es una entrada.
    const SOURCES: &str = r#"Source #51
	State: SUSPENDED
	Name: alsa_output.pci-0000_00_1f.3.analog-stereo.monitor
	Description: Monitor of Audio interno
	Mute: no
	Volume: front-left: 65536 / 100% / 0,00 dB
	Properties:
		device.class = "monitor"

Source #53
	State: RUNNING
	Name: alsa_input.pci-0000_00_1f.3.analog-stereo
	Description: Micrófono interno
	Mute: yes
	Volume: front-left: 30840 /  47% / -19,64 dB
	Properties:
		device.class = "sound"
"#;

    #[test]
    fn las_entradas_de_pactl_no_incluyen_el_monitor_y_usan_el_nombre_como_id() {
        let devices = parse_sources(SOURCES, Some("alsa_input.pci-0000_00_1f.3.analog-stereo"));
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, "alsa_input.pci-0000_00_1f.3.analog-stereo");
        assert_eq!(devices[0].name, "Micrófono interno");
        assert!(devices[0].is_default);
        assert!((devices[0].volume - 0.47).abs() < 1e-9);
    }

    #[test]
    fn el_silencio_se_lee_de_pactl() {
        assert!(parse_mute("Mute: yes\n"));
        assert!(!parse_mute("Mute: no\n"));
        assert!(!parse_mute(""));
    }

    #[test]
    fn el_volumen_del_microfono_se_lee_de_pactl() {
        let output = "Volume: mono: 30840 /  47% / -19,64 dB\n        balance 0,00\n";
        assert_eq!(parse_volume_percent(output).unwrap(), 47);
    }

    #[test]
    fn y_el_ecualizador_se_saca_aunque_no_llegue_la_propiedad() {
        let output = "Sink #1\n\tName: vasak-equalizer\n\tDescription: VasakOS Equalizer\n";
        assert!(parse_sinks(output, None).is_empty());
    }
}
