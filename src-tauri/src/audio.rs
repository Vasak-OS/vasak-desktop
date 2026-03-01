use crate::constants::CMD_WPCTL;
use crate::error::{Result, VasakError};
use crate::logger::{log_info, log_error, log_debug};
use crate::structs::{VolumeInfo, AudioDevice};
use crate::utils::CommandExecutor;
use tauri::{AppHandle, Emitter};

/// Obtiene el ID del sink de audio por defecto
fn get_default_sink_id() -> Result<String> {
    log_debug("Obteniendo ID del sink de audio por defecto");
    let status_output = CommandExecutor::run(CMD_WPCTL, &["status"])?;

    // Buscar el sink por defecto
    let mut in_sinks_section = false;
    let default_sink_id = status_output
        .lines()
        .find_map(|line| {
            if line.contains("Sinks:") {
                in_sinks_section = true;
                return None;
            }
            if in_sinks_section && line.contains("Sources:") {
                in_sinks_section = false;
                return None;
            }
            if in_sinks_section && line.contains("*") {
                // Extraer el ID del sink. Formato: "│  *   61. Device Name [vol: 0.30]"
                if let Some(asterisk_pos) = line.find("*") {
                    let after_asterisk = &line[asterisk_pos + 1..];
                    after_asterisk.split_whitespace().find_map(|part| {
                        if let Some(num_part) = part.strip_suffix('.') {
                            if num_part.chars().all(|c| c.is_ascii_digit()) {
                                Some(num_part.to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .ok_or_else(|| {
            log_error("No se encontró el sink de audio por defecto");
            VasakError::NotFound("No se encontró el sink por defecto".to_string())
        })?;

    log_debug(&format!("Sink de audio por defecto: {}", default_sink_id));
    Ok(default_sink_id)
}

/// Obtiene la información actual del volumen del sistema usando wpctl
pub fn get_volume() -> Result<VolumeInfo> {
    log_debug("Obteniendo información del volumen");
    let default_sink_id = get_default_sink_id()?;

    // Obtener información del volumen
    let volume_output = CommandExecutor::run(CMD_WPCTL, &["get-volume", &default_sink_id])?;

    // Parsear la salida: "Volume: 0.50 [MUTED]" o "Volume: 0.50"
    let parts: Vec<&str> = volume_output.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(VasakError::Parse("Formato de volumen inválido".to_string()));
    }

    let volume_float: f64 = parts[1]
        .parse()
        .map_err(|_| VasakError::Parse("No se pudo parsear el volumen".to_string()))?;

    let current = (volume_float * 100.0) as i64;
    let is_muted = volume_output.contains("[MUTED]");

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
    let default_sink_id = get_default_sink_id()?;

    let volume_percent = format!("{}%", volume);
    CommandExecutor::run(CMD_WPCTL, &["set-volume", &default_sink_id, &volume_percent])?;

    // Si se aplicó correctamente, leer estado y notificar al frontend
    if let Ok(info) = get_volume() {
        log_debug(&format!("Volumen actualizado: {}%", info.current));
        let _ = app.emit("volume-changed", info.clone());
    }
    Ok(())
}

/// Alterna el estado de silencio del audio
pub fn toggle_mute(app: AppHandle) -> Result<bool> {
    log_info("Alternando estado de mute");
    let default_sink_id = get_default_sink_id()?;

    // Obtener estado actual
    let current_info = get_volume()?;

    // Toggle mute
    CommandExecutor::run(CMD_WPCTL, &["set-mute", &default_sink_id, "toggle"])?;
    
    // Después del toggle, obtener estado actualizado y notificar al frontend
    if let Ok(info) = get_volume() {
        log_debug(&format!("Mute actualizado: {}", info.is_muted));
        let _ = app.emit("volume-changed", info.clone());
    }
    
    // Retornar el nuevo estado (opuesto al actual)
    Ok(!current_info.is_muted)
}

/// Lista todos los dispositivos de salida de audio (sinks)
pub fn list_audio_devices() -> Result<Vec<AudioDevice>> {
    log_debug("Listando dispositivos de audio");
    let status_output = CommandExecutor::run(CMD_WPCTL, &["status"])?;
    let default_sink_id = get_default_sink_id().ok();
    
    let mut devices = Vec::new();
    let mut in_sinks_section = false;
    
    for line in status_output.lines() {
        if line.contains("Sinks:") {
            in_sinks_section = true;
            continue;
        }
        if in_sinks_section && line.contains("Sources:") {
            break;
        }
        
        if in_sinks_section && (line.contains("*") || line.contains("│")) {
            // Parse line: "│  *   61. ALSA Playback [vol: 0.30]"
            if let Some(dot_pos) = line.find('.') {
                // Extract ID (number before the dot)
                let before_dot = &line[..dot_pos];
                if let Some(id_str) = before_dot.split_whitespace().last() {
                    let id = id_str.to_string();
                    // Extract device name (everything after the dot and before bracket)
                    let after_dot = &line[dot_pos + 1..];
                    let name = if let Some(bracket_pos) = after_dot.find('[') {
                        after_dot[..bracket_pos].trim().to_string()
                    } else {
                        after_dot.trim().to_string()
                    };
                    
                    // Extract volume if available
                    let volume = if let Some(vol_start) = after_dot.find("vol: ") {
                        let vol_str = &after_dot[vol_start + 5..];
                        vol_str.split([']', ' '])
                            .next()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.5)
                    } else {
                        0.5
                    };
                    
                    let is_default = default_sink_id.as_ref().map(|d| d == &id).unwrap_or(false);
                    
                    devices.push(AudioDevice {
                        id: id.clone(),
                        name: name.clone(),
                        description: name,
                        is_default,
                        volume,
                    });
                }
            }
        }
    }
    
    log_debug(&format!("Encontrados {} dispositivos de audio", devices.len()));
    Ok(devices)
}

/// Establece el dispositivo de salida de audio por defecto
pub fn set_default_audio_device(device_id: &str, app: AppHandle) -> Result<()> {
    log_info(&format!("Estableciendo dispositivo de audio por defecto: {}", device_id));
    CommandExecutor::run(CMD_WPCTL, &["set-default", device_id])?;
    
    // Notify frontend of change
    if let Ok(devices) = list_audio_devices() {
        log_debug("Notificando cambio de dispositivos de audio al frontend");
        let _ = app.emit("audio-devices-changed", devices);
    }
    
    log_info("Dispositivo de audio por defecto establecido correctamente");
    Ok(())
}