use crate::structs::{VolumeInfo, AudioDevice};
use std::process::Command;
use tauri::{AppHandle, Emitter};

// Función helper para ejecutar comandos de PipeWire/PulseAudio
fn run_command(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute {}: {}", cmd, e))?;

    if !output.status.success() {
        return Err(format!(
            "Command {} failed: {}",
            cmd,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// Función helper para encontrar el ID del sink por defecto
fn get_default_sink_id() -> Result<String, String> {
    let status_output = run_command("wpctl", &["status"])?;

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
                // Buscar el número que viene después del asterisco y antes del punto
                if let Some(asterisk_pos) = line.find("*") {
                    let after_asterisk = &line[asterisk_pos + 1..];
                    // Buscar el primer número en la línea después del asterisco
                    after_asterisk.split_whitespace().find_map(|part| {
                        if part.ends_with('.') {
                            // Remover el punto final y verificar que sea un número
                            let num_part = &part[..part.len() - 1];
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
        .ok_or("No se encontró el sink por defecto")?;

    Ok(default_sink_id)
}

pub fn get_volume() -> Result<VolumeInfo, String> {
    // Obtener el sink por defecto usando la función helper
    let default_sink_id = get_default_sink_id()?;

    // Obtener información del volumen
    let volume_output = run_command("wpctl", &["get-volume", &default_sink_id])?;

    // Parsear la salida: "Volume: 0.50 [MUTED]" o "Volume: 0.50"
    let parts: Vec<&str> = volume_output.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Formato de volumen inválido".to_string());
    }

    let volume_float: f64 = parts[1]
        .parse()
        .map_err(|_| "No se pudo parsear el volumen")?;

    let current = (volume_float * 100.0) as i64;
    let is_muted = volume_output.contains("[MUTED]");

    Ok(VolumeInfo {
        current,
        min: 0,
        max: 100,
        is_muted,
    })
}

pub fn set_volume(volume: i64, app: AppHandle) -> Result<(), String> {
    // Obtener el sink por defecto usando la función helper
    let default_sink_id = get_default_sink_id()?;

    let volume_percent = format!("{}%", volume);
    run_command("wpctl", &["set-volume", &default_sink_id, &volume_percent])?;

    // Si se aplicó correctamente, leer estado y notificar al frontend
    if let Ok(info) = get_volume() {
        let _ = app.emit("volume-changed", info.clone());
    }
    Ok(())
}

pub fn toggle_mute(app: AppHandle) -> Result<bool, String> {
    // Obtener el sink por defecto usando la función helper
    let default_sink_id = get_default_sink_id()?;

    // Obtener estado actual
    let current_info = get_volume()?;

    // Toggle mute
    run_command("wpctl", &["set-mute", &default_sink_id, "toggle"])?;
    // Después del toggle, obtener estado actualizado y notificar al frontend
    if let Ok(info) = get_volume() {
        let _ = app.emit("volume-changed", info.clone());
    }
    // Retornar el nuevo estado (opuesto al actual)
    Ok(!current_info.is_muted)
}
/// List all audio output devices (sinks)
pub fn list_audio_devices() -> Result<Vec<AudioDevice>, String> {
    let status_output = run_command("wpctl", &["status"])?;
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
                        let vol_val = vol_str.split(|c| c == ']' || c == ' ')
                            .next()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.5);
                        vol_val
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
    
    Ok(devices)
}

/// Set the default audio output device
pub fn set_default_audio_device(device_id: &str, app: AppHandle) -> Result<(), String> {
    run_command("wpctl", &["set-default", device_id])?;
    
    // Notify frontend of change
    if let Ok(devices) = list_audio_devices() {
        let _ = app.emit("audio-devices-changed", devices);
    }
    
    Ok(())
}