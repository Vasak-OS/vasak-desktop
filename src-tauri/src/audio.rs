use crate::constants::CMD_PACTL;
use crate::error::{Result, VasakError};
use crate::logger::{log_info, log_error, log_debug};
use crate::structs::{VolumeInfo, AudioDevice};
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
            let trimmed = line.trim();
            if let Some(suffix) = trimmed.strip_prefix("Default Sink:") {
                Some(suffix.trim().to_string())
            } else if let Some(suffix) = trimmed.strip_prefix("default sink:") {
                Some(suffix.trim().to_string())
            } else {
                None
            }
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
            if let Some(pct) = trimmed.split_whitespace().find_map(|part| {
                part.strip_suffix('%').and_then(|s| s.parse::<i64>().ok())
            }) {
                volume_pct = Some(pct);
            }
        }
    }

    // Si no encontramos datos del sink por defecto, intentar parse global
    let current = volume_pct.or_else(|| {
        parse_volume_percent(&list_output).ok()
    }).unwrap_or(0);

    Ok(VolumeInfo {
        current,
        min: 0,
        max: 100,
        is_muted,
    })
}

/// Establece el volumen del sistema
pub fn set_volume(volume: i64, app: AppHandle) -> Result<()> {
    log_info(&format!("Estableciendo volumen a: {}%", volume));
    let sink = get_default_sink_name()?;
    let volume_str = format!("{}%", volume);

    CommandExecutor::run(CMD_PACTL, &["set-sink-volume", &sink, &volume_str])?;

    if let Ok(info) = get_volume() {
        log_debug(&format!("Volumen actualizado: {}%", info.current));
        let _ = app.emit("volume-changed", info.clone());
    }
    Ok(())
}

/// Alterna el estado de silencio del audio
pub fn toggle_mute(app: AppHandle) -> Result<bool> {
    log_info("Alternando estado de mute");
    let sink = get_default_sink_name()?;
    let current_info = get_volume()?;

    CommandExecutor::run(CMD_PACTL, &["set-sink-mute", &sink, "toggle"])?;

    if let Ok(info) = get_volume() {
        log_debug(&format!("Mute actualizado: {}", info.is_muted));
        let _ = app.emit("volume-changed", info.clone());
    }

    Ok(!current_info.is_muted)
}

/// Lista todos los dispositivos de salida de audio (sinks)
pub fn list_audio_devices() -> Result<Vec<AudioDevice>> {
    log_debug("Listando dispositivos de audio");
    let output = CommandExecutor::run(CMD_PACTL, &["list", "sinks"])?;
    let default_sink = get_default_sink_name().ok();

    let mut devices = Vec::new();
    let mut current_id = String::new();
    let mut current_name = String::new();
    let mut current_description = String::new();
    let mut current_volume = 0.5;
    let mut in_sink = false;

    for line in output.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("Sink #") {
            // Guardar sink anterior
            if in_sink && !current_id.is_empty() {
                let desc = if current_description.is_empty() {
                    current_name.clone()
                } else {
                    current_description.clone()
                };
                let is_default = default_sink.as_ref().map(|d| d == &current_name).unwrap_or(false);
                devices.push(AudioDevice {
                    id: current_id.clone(),
                    name: desc,
                    description: current_name.clone(),
                    is_default,
                    volume: current_volume,
                });
            }

            // Nuevo sink
            current_id = rest.split_whitespace().next().unwrap_or("").to_string();
            current_name.clear();
            current_description.clear();
            current_volume = 0.5;
            in_sink = true;
            continue;
        }

        if !in_sink {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Name:") {
            current_name = rest.trim().to_string();
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Description:") {
            current_description = rest.trim().to_string();
            continue;
        }

        if trimmed.starts_with("Volume:") || trimmed.starts_with("volume:") {
            if let Some(pct) = trimmed.split_whitespace().find_map(|part| {
                part.strip_suffix('%').and_then(|s| s.parse::<f64>().ok())
            }) {
                current_volume = pct / 100.0;
            }
            continue;
        }

        // Fin de este sink (siguiente sink o línea vacía separadora)
        if trimmed.is_empty() && in_sink && !current_name.is_empty() {
            let desc = if current_description.is_empty() {
                current_name.clone()
            } else {
                current_description.clone()
            };
            let is_default = default_sink.as_ref().map(|d| d == &current_name).unwrap_or(false);
            devices.push(AudioDevice {
                id: current_id.clone(),
                name: desc,
                description: current_name.clone(),
                is_default,
                volume: current_volume,
            });
            current_id.clear();
            current_name.clear();
            current_description.clear();
            current_volume = 0.5;
            in_sink = false;
        }
    }

    // Último sink
    if in_sink && !current_id.is_empty() {
        let desc = if current_description.is_empty() {
            current_name.clone()
        } else {
            current_description.clone()
        };
        let is_default = default_sink.as_ref().map(|d| d == &current_name).unwrap_or(false);
        devices.push(AudioDevice {
            id: current_id.clone(),
            name: desc,
            description: current_name.clone(),
            is_default,
            volume: current_volume,
        });
    }

    log_debug(&format!("Encontrados {} dispositivos de audio", devices.len()));
    Ok(devices)
}

/// Establece el dispositivo de salida de audio por defecto
pub fn set_default_audio_device(device_id: &str, app: AppHandle) -> Result<()> {
    log_info(&format!("Estableciendo dispositivo de audio por defecto: {}", device_id));
    CommandExecutor::run(CMD_PACTL, &["set-default-sink", device_id])?;

    clear_sink_cache();

    if let Ok(devices) = list_audio_devices() {
        log_debug("Notificando cambio de dispositivos de audio al frontend");
        let _ = app.emit("audio-devices-changed", devices);
    }

    log_info("Dispositivo de audio por defecto establecido correctamente");
    Ok(())
}
