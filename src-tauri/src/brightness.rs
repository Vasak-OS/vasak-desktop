use std::process::Command;
use std::str::from_utf8;
use std::path::Path;
use std::fs;
use crate::structs::BrightnessInfo;

/// Detecta si estamos en Wayland
fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

/// Intenta obtener brillo usando brightnessctl
fn get_brightness_brightnessctl() -> Result<BrightnessInfo, String> {
    let current_output = Command::new("brightnessctl")
        .arg("get")
        .output()
        .map_err(|e| format!("brightnessctl not available: {}", e))?;

    if !current_output.status.success() {
        return Err("brightnessctl get failed".to_string());
    }

    let max_output = Command::new("brightnessctl")
        .arg("max")
        .output()
        .map_err(|e| format!("brightnessctl max failed: {}", e))?;

    if !max_output.status.success() {
        return Err("brightnessctl max failed".to_string());
    }

    let current: u32 = from_utf8(&current_output.stdout)
        .map_err(|e| format!("Invalid UTF-8: {}", e))?
        .trim()
        .parse()
        .map_err(|e| format!("Parse error: {}", e))?;

    let max: u32 = from_utf8(&max_output.stdout)
        .map_err(|e| format!("Invalid UTF-8: {}", e))?
        .trim()
        .parse()
        .map_err(|e| format!("Parse error: {}", e))?;

    // Convertir a porcentaje
    let current_percent = if max > 0 {
        ((current as f32 / max as f32) * 100.0) as u32
    } else {
        0
    };

    Ok(BrightnessInfo {
        current: current_percent,
        max: 100,
        min: 0,
    })
}

/// Intenta obtener brillo usando sysfs
fn get_brightness_sysfs() -> Result<BrightnessInfo, String> {
    let backlight_path = "/sys/class/backlight";
    
    if !Path::new(backlight_path).exists() {
        return Err("No backlight devices found".to_string());
    }

    // Buscar el primer dispositivo de backlight
    let entries = fs::read_dir(backlight_path)
        .map_err(|e| format!("Cannot read backlight dir: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Error reading entry: {}", e))?;
        let device_path = entry.path();

        let brightness_file = device_path.join("brightness");
        let max_brightness_file = device_path.join("max_brightness");

        if brightness_file.exists() && max_brightness_file.exists() {
            let current: u32 = fs::read_to_string(&brightness_file)
                .map_err(|e| format!("Cannot read brightness: {}", e))?
                .trim()
                .parse()
                .map_err(|e| format!("Parse error: {}", e))?;

            let max: u32 = fs::read_to_string(&max_brightness_file)
                .map_err(|e| format!("Cannot read max_brightness: {}", e))?
                .trim()
                .parse()
                .map_err(|e| format!("Parse error: {}", e))?;

            // Convertir a porcentaje
            let current_percent = if max > 0 {
                ((current as f32 / max as f32) * 100.0) as u32
            } else {
                0
            };

            return Ok(BrightnessInfo {
                current: current_percent,
                max: 100,
                min: 0,
            });
        }
    }

    Err("No valid backlight device found".to_string())
}

/// Intenta obtener brillo usando xrandr (X11 only)
fn get_brightness_xrandr() -> Result<BrightnessInfo, String> {
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

    let current_brightness = output_str
        .lines()
        .find(|line| line.contains("Brightness:"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<f32>().ok())
        .ok_or_else(|| "Could not find current brightness".to_string())?;

    let current = (current_brightness * 100.0) as u32;

    Ok(BrightnessInfo {
        current,
        max: 100,
        min: 0,
    })
}

/// Obtiene el brillo actual usando el mejor método disponible
pub fn get_brightness() -> Result<BrightnessInfo, String> {
    // Estrategia de fallback:
    // 1. Intentar brightnessctl (funciona en Wayland y X11)
    // 2. Intentar sysfs directo
    // 3. Intentar xrandr si estamos en X11
    
    if let Ok(info) = get_brightness_brightnessctl() {
        return Ok(info);
    }

    if let Ok(info) = get_brightness_sysfs() {
        return Ok(info);
    }

    if !is_wayland() {
        if let Ok(info) = get_brightness_xrandr() {
            return Ok(info);
        }
    }

    Err("No brightness control method available".to_string())
}

/// Establece el brillo usando brightnessctl
fn set_brightness_brightnessctl(brightness: u32) -> Result<(), String> {
    let brightness_str = format!("{}%", brightness);
    
    let output = Command::new("brightnessctl")
        .args(["set", &brightness_str])
        .output()
        .map_err(|e| format!("brightnessctl not available: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "brightnessctl set failed: {}",
            from_utf8(&output.stderr).unwrap_or("Unknown error")
        ));
    }

    Ok(())
}

/// Establece el brillo usando sysfs
fn set_brightness_sysfs(brightness: u32) -> Result<(), String> {
    let backlight_path = "/sys/class/backlight";
    
    if !Path::new(backlight_path).exists() {
        return Err("No backlight devices found".to_string());
    }

    let entries = fs::read_dir(backlight_path)
        .map_err(|e| format!("Cannot read backlight dir: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Error reading entry: {}", e))?;
        let device_path = entry.path();

        let brightness_file = device_path.join("brightness");
        let max_brightness_file = device_path.join("max_brightness");

        if brightness_file.exists() && max_brightness_file.exists() {
            let max: u32 = fs::read_to_string(&max_brightness_file)
                .map_err(|e| format!("Cannot read max_brightness: {}", e))?
                .trim()
                .parse()
                .map_err(|e| format!("Parse error: {}", e))?;

            // Convertir porcentaje a valor absoluto
            let absolute_value = ((brightness as f32 / 100.0) * max as f32) as u32;

            fs::write(&brightness_file, absolute_value.to_string())
                .map_err(|e| format!("Cannot write brightness (check permissions): {}", e))?;

            return Ok(());
        }
    }

    Err("No valid backlight device found".to_string())
}

/// Establece el brillo usando xrandr (X11 only)
fn set_brightness_xrandr(brightness: u32) -> Result<(), String> {
    let brightness_value = (brightness as f32 / 100.0).to_string();

    let output = Command::new("xrandr")
        .args(["--listmonitors"])
        .output()
        .map_err(|e| format!("Error getting monitors: {}", e))?;

    let monitors_str =
        from_utf8(&output.stdout).map_err(|e| format!("Invalid UTF-8 in output: {}", e))?;

    let monitors: Vec<&str> = monitors_str
        .lines()
        .skip(1)
        .filter_map(|line| line.split_whitespace().last())
        .collect();

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

/// Establece el brillo usando el mejor método disponible
pub fn set_brightness(brightness: u32) -> Result<(), String> {
    // Estrategia de fallback:
    // 1. Intentar brightnessctl (funciona en Wayland y X11)
    // 2. Intentar sysfs directo
    // 3. Intentar xrandr si estamos en X11
    
    if set_brightness_brightnessctl(brightness).is_ok() {
        return Ok(());
    }

    if set_brightness_sysfs(brightness).is_ok() {
        return Ok(());
    }

    if !is_wayland() {
        if set_brightness_xrandr(brightness).is_ok() {
            return Ok(());
        }
    }

    Err("No brightness control method available. Try installing 'brightnessctl' or check permissions for /sys/class/backlight".to_string())
}
