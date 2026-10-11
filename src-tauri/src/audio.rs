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
    /// La `card` dueña del sink, de la propiedad `device.name`. Sirve para
    /// casar el sink con su card al conmutar un perfil.
    card: String,
}

impl SinkDraft {
    fn finish(self, default_sink: Option<&str>) -> Option<AudioDevice> {
        if self.id.is_empty() || self.smart_filter || self.monitor || self.name == EQUALIZER_SINK {
            return None;
        }
        let name = if self.description.is_empty() {
            self.name.clone()
        } else {
            self.description
        };
        let card = if self.card.is_empty() {
            None
        } else {
            Some(self.card)
        };
        Some(AudioDevice {
            id: self.id,
            is_default: default_sink == Some(self.name.as_str()),
            name,
            description: self.name,
            volume: self.volume.unwrap_or(0.5),
            card,
            profile: None,
            active: true,
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
        } else if let Some(value) = trimmed.strip_prefix("device.name") {
            // `device.name = "alsa_card..."`: la card dueña del sink.
            draft.card = value
                .trim_start()
                .trim_start_matches('=')
                .trim()
                .trim_matches('"')
                .to_string();
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

// ── Las salidas disponibles-pero-inactivas (vasak-desktop#230) ───────────────
//
// `pactl list sinks` sólo trae los sinks que ya existen. El sink HDMI de la
// placa no existe mientras el perfil de su `card` esté en `off` o en analógico,
// así que la salida HDMI no aparece en el selector. Para poder elegirla hay que
// enumerar los perfiles de salida de cada `card` y conmutar el que falte.

/// Un perfil de salida de una `card`, de la línea de `Profiles:`.
#[derive(Debug, Clone, PartialEq)]
struct CardProfile {
    /// El identificador que toma `pactl set-card-profile` (`output:hdmi-stereo`).
    name: String,
    /// Lo que se le muestra a quien usa el sistema («Salida digital (HDMI)»).
    description: String,
    /// Cuántos sinks crea el perfil. Sin sinks no es una salida.
    sinks: u32,
    /// `available: no` cuando no hay nada conectado (HDMI sin monitor): no se
    /// ofrece.
    available: bool,
}

/// Una `card` de `pactl list cards`, con sus perfiles de salida.
#[derive(Debug, Clone, PartialEq, Default)]
struct CardInfo {
    /// El `Name` de la card (`alsa_card.pci-0000_01_00.1`), lo que casa con la
    /// propiedad `device.name` de sus sinks.
    name: String,
    /// La descripción de la card (propiedad `device.description`).
    description: String,
    /// El perfil activo; su sink ya aparece en `pactl list sinks`, así que no
    /// se vuelve a ofrecer como salida inactiva.
    active_profile: String,
    profiles: Vec<CardProfile>,
}

/// Parsea una línea de perfil de `pactl list cards`, p. ej.
/// `output:hdmi-stereo: Salida digital (HDMI) (sinks: 1, sources: 0, priority: 5900, available: yes)`.
fn parse_card_profile(line: &str) -> Option<CardProfile> {
    let (name, rest) = line.split_once(": ")?;
    // Los metadatos van siempre al final, entre paréntesis: ` (sinks: N, ...)`.
    let (description, meta) = rest.rsplit_once(" (sinks:")?;
    let sinks = meta
        .trim_start()
        .split(|c: char| !c.is_ascii_digit())
        .find(|s| !s.is_empty())
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let available = meta
        .split_once("available:")
        .map(|(_, value)| !value.trim_start().starts_with("no"))
        .unwrap_or(true);
    Some(CardProfile {
        name: name.trim().to_string(),
        description: description.trim().to_string(),
        sinks,
        available,
    })
}

/// Las `card`s y sus perfiles de salida, a partir de `pactl list cards`.
fn parse_cards(output: &str) -> Vec<CardInfo> {
    let mut cards = Vec::new();
    let mut current: Option<CardInfo> = None;
    let mut in_profiles = false;

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("Card #") {
            if let Some(card) = current.take() {
                cards.push(card);
            }
            current = Some(CardInfo::default());
            in_profiles = false;
            continue;
        }

        let Some(card) = current.as_mut() else {
            continue;
        };

        if trimmed == "Profiles:" {
            in_profiles = true;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Active Profile:") {
            card.active_profile = rest.trim().to_string();
            in_profiles = false;
            continue;
        }

        if trimmed.starts_with("Ports:") || trimmed.starts_with("Name:") {
            // El `Name:` de la card va antes de `Profiles:`; `Ports:` después.
            if let Some(rest) = trimmed.strip_prefix("Name:") {
                if card.name.is_empty() {
                    card.name = rest.trim().to_string();
                }
            }
            in_profiles = false;
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("device.description") {
            card.description = value
                .trim_start()
                .trim_start_matches('=')
                .trim()
                .trim_matches('"')
                .to_string();
            continue;
        }

        if in_profiles {
            if let Some(profile) = parse_card_profile(trimmed) {
                card.profiles.push(profile);
            }
        }
    }

    if let Some(card) = current.take() {
        cards.push(card);
    }
    cards
}

/// Las salidas disponibles-pero-inactivas, a partir de las `card`s.
///
/// Se ofrece un perfil cuando crea al menos un sink, está disponible, es un
/// perfil de salida pura (`output:` sin `+input`, para no duplicar cada salida
/// con su variante «+ Entrada») y no es el perfil activo (su sink ya está en la
/// lista). Así una placa HDMI en `off` aparece con sus salidas HDMI reales, sin
/// llenar la lista con cada combinación.
fn card_output_devices(cards: &[CardInfo]) -> Vec<AudioDevice> {
    let mut devices = Vec::new();
    for card in cards {
        for profile in &card.profiles {
            if profile.sinks == 0
                || !profile.available
                || profile.name == card.active_profile
                || !profile.name.starts_with("output:")
                || profile.name.contains("+input")
            {
                continue;
            }
            let card_label = if card.description.is_empty() {
                card.name.as_str()
            } else {
                card.description.as_str()
            };
            let name = if card_label.is_empty() {
                profile.description.clone()
            } else {
                format!("{} — {}", card_label, profile.description)
            };
            devices.push(AudioDevice {
                // Único y estable: la misma card+perfil siempre da el mismo id.
                id: format!("{}::{}", card.name, profile.name),
                name,
                description: profile.name.clone(),
                is_default: false,
                volume: 0.0,
                card: Some(card.name.clone()),
                profile: Some(profile.name.clone()),
                active: false,
            });
        }
    }
    devices
}

/// El sink que quedó asociado a una `card` tras conmutar su perfil.
fn find_card_sink<'a>(devices: &'a [AudioDevice], card: &str) -> Option<&'a AudioDevice> {
    devices
        .iter()
        .find(|d| d.active && d.card.as_deref() == Some(card))
}

/// Las salidas de audio: los sinks que ya existen más las salidas de `card`
/// disponibles pero todavía sin sink (HDMI con la placa en otro perfil).
pub fn list_audio_outputs() -> Result<Vec<AudioDevice>> {
    log_debug("Listando salidas de audio (sinks + perfiles de card)");
    let mut devices = list_audio_devices()?;
    let cards_output = CommandExecutor::run(CMD_PACTL, &["list", "cards"])?;
    devices.extend(card_output_devices(&parse_cards(&cards_output)));
    log_debug(&format!("Encontradas {} salidas de audio", devices.len()));
    Ok(devices)
}

/// Elige una salida de audio.
///
/// Si la salida ya es un sink (`sink_id`), fija ese sink por omisión. Si es una
/// salida inactiva de una `card` (`card` + `profile`), conmuta primero el perfil
/// de la placa —así se crea el sink HDMI— y después fija ese sink nuevo.
pub fn set_audio_output(
    sink_id: Option<&str>,
    card: Option<&str>,
    profile: Option<&str>,
    app: AppHandle,
) -> Result<()> {
    match (card, profile) {
        (Some(card), Some(profile)) => {
            log_info(&format!(
                "Conmutando el perfil de la card {} a {}",
                card, profile
            ));
            CommandExecutor::run(CMD_PACTL, &["set-card-profile", card, profile])?;
            clear_sink_cache();

            let sinks_output = CommandExecutor::run(CMD_PACTL, &["list", "sinks"])?;
            let sinks = parse_sinks(&sinks_output, None);
            let sink = find_card_sink(&sinks, card).ok_or_else(|| {
                log_error(&format!(
                    "No apareció ningún sink para la card {} tras conmutar a {}",
                    card, profile
                ));
                VasakError::NotFound(format!("No hay sink para la card {}", card))
            })?;
            CommandExecutor::run(CMD_PACTL, &["set-default-sink", &sink.description])?;
        }
        _ => {
            let sink_id = sink_id.ok_or_else(|| {
                VasakError::NotFound("Falta el sink o el perfil de la salida".to_string())
            })?;
            CommandExecutor::run(CMD_PACTL, &["set-default-sink", sink_id])?;
        }
    }

    clear_sink_cache();

    if let Ok(devices) = list_audio_outputs() {
        log_debug("Notificando cambio de salidas de audio al frontend");
        let _ = app.emit("audio-devices-changed", devices);
    }

    log_info("Salida de audio establecida correctamente");
    Ok(())
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
    Ok(parse_sources(
        &output,
        default_source.as_deref().map(str::trim),
    ))
}

/// Elige la entrada por omisión, por el nombre del nodo.
///
/// Con el flujo de `pw-dump` el cambio vuelve solo, como
/// `audio-input-devices-changed` desde el applet de audio; sin él se avisa
/// acá, para que el selector no quede viejo.
pub fn set_default_audio_input_device(device_id: &str, app: AppHandle) -> Result<()> {
    log_info(&format!(
        "Estableciendo la entrada de audio por defecto: {}",
        device_id
    ));
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

    // ── Salidas disponibles-pero-inactivas: HDMI con la placa en otro perfil ──
    // (vasak-desktop#230)

    /// Lo que imprime `pactl list cards` con dos placas: la interna en
    /// analógico y la del HDMI con el perfil en `off` (su sink todavía no
    /// existe). Recortado a lo que se lee.
    const CARDS: &str = r#"Card #0
	Name: alsa_card.pci-0000_00_1f.3
	Driver: alsa
	Owner Module: 7
	Properties:
		alsa.card = "0"
		device.description = "Audio interno"
	Profiles:
		output:analog-stereo: Estéreo analógico (sinks: 1, sources: 0, priority: 6500, available: yes)
		output:analog-stereo+input:analog-stereo: Estéreo analógico + Entrada (sinks: 1, sources: 1, priority: 6600, available: yes)
		output:hdmi-stereo: Salida digital (HDMI) (sinks: 1, sources: 0, priority: 5400, available: no)
		off: Apagado (sinks: 0, sources: 0, priority: 0, available: yes)
	Active Profile: output:analog-stereo
	Ports:
		analog-output-speaker: Altavoces (priority: 10000, latency offset: 0 usec)

Card #1
	Name: alsa_card.pci-0000_01_00.1
	Driver: alsa
	Owner Module: 9
	Properties:
		alsa.card = "1"
		device.description = "Audio NVIDIA"
	Profiles:
		output:hdmi-stereo: Salida digital (HDMI) (sinks: 1, sources: 0, priority: 5900, available: yes)
		output:hdmi-stereo-extra1: Salida digital (HDMI 2) (sinks: 1, sources: 0, priority: 5700, available: no)
		output:hdmi-surround71+input:analog-stereo: Envolvente 7.1 + Entrada (sinks: 1, sources: 1, priority: 100, available: yes)
		off: Apagado (sinks: 0, sources: 0, priority: 0, available: yes)
	Active Profile: off
	Ports:
		hdmi-output-0: HDMI / DisplayPort (priority: 5900, latency offset: 0 usec)
"#;

    #[test]
    fn un_perfil_hdmi_se_parsea_con_su_nombre_descripcion_sinks_y_disponibilidad() {
        let line =
            "output:hdmi-stereo: Salida digital (HDMI) (sinks: 1, sources: 0, priority: 5900, available: yes)";
        let profile = parse_card_profile(line).expect("la línea es un perfil válido");
        assert_eq!(profile.name, "output:hdmi-stereo");
        // La descripción conserva sus propios paréntesis y no se come el `(HDMI)`.
        assert_eq!(profile.description, "Salida digital (HDMI)");
        assert_eq!(profile.sinks, 1);
        assert!(profile.available);
    }

    #[test]
    fn un_perfil_sin_nada_conectado_queda_como_no_disponible() {
        let line =
            "output:hdmi-stereo-extra1: Salida digital (HDMI 2) (sinks: 1, sources: 0, priority: 5700, available: no)";
        let profile = parse_card_profile(line).expect("la línea es un perfil válido");
        assert!(!profile.available);
    }

    #[test]
    fn parse_cards_lee_la_placa_hdmi_con_su_perfil_en_off() {
        let cards = parse_cards(CARDS);
        assert_eq!(cards.len(), 2);

        let nvidia = cards
            .iter()
            .find(|c| c.name == "alsa_card.pci-0000_01_00.1")
            .expect("la placa HDMI está en la lista aunque su perfil esté en off");
        assert_eq!(nvidia.description, "Audio NVIDIA");
        assert_eq!(nvidia.active_profile, "off");
        // El HDMI inactivo aparece entre los perfiles de la placa.
        assert!(nvidia
            .profiles
            .iter()
            .any(|p| p.name == "output:hdmi-stereo" && p.available));
    }

    #[test]
    fn card_output_devices_ofrece_la_salida_hdmi_inactiva_pero_disponible() {
        let cards = parse_cards(CARDS);
        let outputs = card_output_devices(&cards);

        // Sólo el HDMI real de la placa NVIDIA: el `extra1` no está conectado
        // (available: no), el `off` no crea sink, el envolvente es `+input` y
        // los perfiles de la placa interna son el activo o `+input`.
        assert_eq!(outputs.len(), 1, "salidas ofrecidas: {:?}", outputs);
        let hdmi = &outputs[0];
        assert_eq!(hdmi.id, "alsa_card.pci-0000_01_00.1::output:hdmi-stereo");
        assert_eq!(hdmi.name, "Audio NVIDIA — Salida digital (HDMI)");
        assert_eq!(hdmi.card.as_deref(), Some("alsa_card.pci-0000_01_00.1"));
        assert_eq!(hdmi.profile.as_deref(), Some("output:hdmi-stereo"));
        // Es una salida sin sink todavía: la UI la marca distinto y, al
        // elegirla, hay que conmutar el perfil antes de fijarla.
        assert!(!hdmi.active);
        assert!(!hdmi.is_default);
    }

    #[test]
    fn card_output_devices_no_duplica_una_salida_con_su_variante_de_entrada() {
        let cards = parse_cards(CARDS);
        let outputs = card_output_devices(&cards);
        // Ningún perfil ofrecido es un `+input`: no se duplica cada salida con
        // su combinación «+ Entrada».
        assert!(outputs.iter().all(|d| !d.description.contains("+input")));
    }

    #[test]
    fn card_output_devices_no_reofrece_el_perfil_activo() {
        let cards = parse_cards(CARDS);
        let outputs = card_output_devices(&cards);
        // El analógico de la placa interna es el perfil activo: su sink ya
        // está en `pactl list sinks`, así que no se vuelve a ofrecer.
        assert!(outputs
            .iter()
            .all(|d| d.profile.as_deref() != Some("output:analog-stereo")));
    }

    /// Lo que imprime `pactl list sinks` después de conmutar el perfil HDMI:
    /// aparece el sink nuevo, que trae la `card` en `device.name`.
    const SINKS_TRAS_CONMUTAR: &str = r#"Sink #52
	State: SUSPENDED
	Name: alsa_output.pci-0000_00_1f.3.analog-stereo
	Description: Audio interno Estéreo analógico
	Volume: front-left: 39321 /  60% / -13,31 dB
	Properties:
		device.name = "alsa_card.pci-0000_00_1f.3"

Sink #99
	State: IDLE
	Name: alsa_output.pci-0000_01_00.1.hdmi-stereo
	Description: Salida digital (HDMI)
	Volume: front-left: 65536 / 100% / 0,00 dB
	Properties:
		device.name = "alsa_card.pci-0000_01_00.1"
"#;

    #[test]
    fn tras_conmutar_el_perfil_se_encuentra_el_sink_nuevo_por_su_card() {
        // Es el paso central de `set_audio_output` con card+profile: conmutado
        // el perfil, el sink HDMI ya existe y se lo ubica por `device.name`.
        let sinks = parse_sinks(SINKS_TRAS_CONMUTAR, None);
        let sink = find_card_sink(&sinks, "alsa_card.pci-0000_01_00.1")
            .expect("el sink HDMI aparece tras conmutar el perfil");
        // `set-default-sink` recibe el `Name` del sink, que queda en `description`.
        assert_eq!(sink.description, "alsa_output.pci-0000_01_00.1.hdmi-stereo");
    }

    #[test]
    fn sin_sink_para_la_card_no_se_inventa_ninguno() {
        // Si tras conmutar no apareció ningún sink de esa placa, no se casa con
        // el de otra: `set_audio_output` devuelve NotFound en lugar de fijar la
        // salida equivocada.
        let sinks = parse_sinks(SINKS_TRAS_CONMUTAR, None);
        assert!(find_card_sink(&sinks, "alsa_card.pci-9999_99_99.9").is_none());
    }
}
