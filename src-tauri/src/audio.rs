use std::process::Command;
use crate::structs::VolumeInfo;

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
                    after_asterisk
                        .split_whitespace()
                        .find_map(|part| {
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

#[tauri::command]
pub fn set_volume(volume: i64) -> Result<(), String> {
    // Obtener el sink por defecto usando la función helper
    let default_sink_id = get_default_sink_id()?;

    let volume_percent = format!("{}%", volume);
    run_command("wpctl", &["set-volume", &default_sink_id, &volume_percent])?;
    
    Ok(())
}

#[tauri::command]
pub fn toggle_mute() -> Result<bool, String> {
    // Obtener el sink por defecto usando la función helper
    let default_sink_id = get_default_sink_id()?;

    // Obtener estado actual
    let current_info = get_volume()?;
    
    // Toggle mute
    run_command("wpctl", &["set-mute", &default_sink_id, "toggle"])?;
    
    // Retornar el nuevo estado (opuesto al actual)
    Ok(!current_info.is_muted)
}

