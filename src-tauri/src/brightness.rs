use std::process::Command;
use std::str::from_utf8;
use crate::structs::BrightnessInfo;

pub fn get_brightness() -> Result<BrightnessInfo, String> {
    // Intentar obtener el brillo actual y máximo usando xrandr
    let output = Command::new("xrandr")
        .args(["--verbose"])
        .output()
        .map_err(|e| format!("Error executing xrandr: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "xrandr command failed: {}",
            from_utf8(&output.stderr).unwrap_or("Unknown error")
        ));
    }

    let output_str =
        from_utf8(&output.stdout).map_err(|e| format!("Invalid UTF-8 in output: {}", e))?;

    // Buscar la línea que contiene "Brightness:"
    let current_brightness = output_str
        .lines()
        .find(|line| line.contains("Brightness:"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<f32>().ok())
        .ok_or_else(|| "Could not find current brightness".to_string())?;

    // Convertir el valor de brillo (0.0-1.0) a porcentaje (0-100)
    let current = (current_brightness * 100.0) as u32;

    Ok(BrightnessInfo {
        current,
        max: 100,
        min: 0,
    })
}

pub fn set_brightness(brightness: u32) -> Result<(), String> {
    // Convertir el porcentaje (0-100) a valor de brillo (0.0-1.0)
    let brightness_value = (brightness as f32 / 100.0).to_string();

    // Obtener la lista de monitores
    let output = Command::new("xrandr")
        .args(["--listmonitors"])
        .output()
        .map_err(|e| format!("Error getting monitors: {}", e))?;

    let monitors_str =
        from_utf8(&output.stdout).map_err(|e| format!("Invalid UTF-8 in output: {}", e))?;

    // Extraer los nombres de los monitores
    let monitors: Vec<&str> = monitors_str
        .lines()
        .skip(1) // Saltar la primera línea que es el encabezado
        .filter_map(|line| line.split_whitespace().last())
        .collect();

    // Aplicar el brillo a cada monitor
    for monitor in monitors {
        let output = Command::new("xrandr")
            .args(["--output", monitor, "--brightness", &brightness_value])
            .output()
            .map_err(|e| format!("Error setting brightness: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Failed to set brightness for {}: {}",
                monitor,
                from_utf8(&output.stderr).unwrap_or("Unknown error")
            ));
        }
    }

    Ok(())
}
