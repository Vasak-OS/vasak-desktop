use std::path::{Path, PathBuf};
use std::fs;
use std::sync::OnceLock;
use crate::constants::{CMD_BRIGHTNESSCTL, CMD_XRANDR, BACKLIGHT_PATH};
use crate::error::{Result, VasakError};
use crate::structs::BrightnessInfo;
use crate::utils::CommandExecutor;

/// Detecta si estamos en Wayland
fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

/// Método de control de brillo disponible
#[derive(Clone)]
enum BrightnessMethod {
    Brightnessctl,
    Sysfs(PathBuf),
    Xrandr,
}

/// Cache global del método de brillo detectado
static BRIGHTNESS_METHOD: OnceLock<BrightnessMethod> = OnceLock::new();

/// Obtiene el método de brillo, detectándolo solo la primera vez
fn get_brightness_method() -> Result<BrightnessMethod> {
    // Si ya está en cache, retornar el valor clonado
    if let Some(method) = BRIGHTNESS_METHOD.get() {
        return Ok(method.clone());
    }
    
    // Detectar y cachear el método
    let method = detect_brightness_method()?;
    
    // Intentar guardar en cache (puede fallar si otro thread lo hizo primero, pero está bien)
    let _ = BRIGHTNESS_METHOD.set(method.clone());
    
    Ok(method)
}

/// Detecta el mejor método de control de brillo disponible
fn detect_brightness_method() -> Result<BrightnessMethod> {
    // 1. Intentar brightnessctl (funciona en Wayland y X11)
    if CommandExecutor::run_silent(CMD_BRIGHTNESSCTL, &["--version"]) {
        return Ok(BrightnessMethod::Brightnessctl);
    }

    // 2. Intentar sysfs directo
    if let Ok(device_path) = find_backlight_device() {
        return Ok(BrightnessMethod::Sysfs(device_path));
    }

    // 3. Intentar xrandr si estamos en X11
    if !is_wayland() && CommandExecutor::run_silent(CMD_XRANDR, &["--version"]) {
        return Ok(BrightnessMethod::Xrandr);
    }

    Err(VasakError::Brightness(
        "No brightness control method available. Try installing 'brightnessctl'".to_string()
    ))
}

/// Encuentra el primer dispositivo de backlight disponible
fn find_backlight_device() -> Result<PathBuf> {
    if !Path::new(BACKLIGHT_PATH).exists() {
        return Err(VasakError::NotFound("No backlight devices found".to_string()));
    }

    let entries = fs::read_dir(BACKLIGHT_PATH)
        .map_err(|e| VasakError::Io(e))?;

    for entry in entries {
        let entry = entry.map_err(|e| VasakError::Io(e))?;
        let device_path = entry.path();

        let brightness_file = device_path.join("brightness");
        let max_brightness_file = device_path.join("max_brightness");

        if brightness_file.exists() && max_brightness_file.exists() {
            return Ok(device_path);
        }
    }

    Err(VasakError::NotFound("No valid backlight device found".to_string()))
}

/// Obtiene el brillo usando brightnessctl
fn get_brightness_brightnessctl() -> Result<BrightnessInfo> {
    let current_output = CommandExecutor::run(CMD_BRIGHTNESSCTL, &["get"])?;
    let max_output = CommandExecutor::run(CMD_BRIGHTNESSCTL, &["max"])?;

    let current: u32 = current_output.trim()
        .parse()
        .map_err(|e| VasakError::Parse(format!("Failed to parse brightness: {}", e)))?;

    let max: u32 = max_output.trim()
        .parse()
        .map_err(|e| VasakError::Parse(format!("Failed to parse max brightness: {}", e)))?;

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

/// Obtiene el brillo usando sysfs
fn get_brightness_sysfs(device_path: &Path) -> Result<BrightnessInfo> {
    let brightness_file = device_path.join("brightness");
    let max_brightness_file = device_path.join("max_brightness");

    let current: u32 = fs::read_to_string(&brightness_file)
        .map_err(|e| VasakError::Io(e))?
        .trim()
        .parse()
        .map_err(|e| VasakError::Parse(format!("Failed to parse brightness: {}", e)))?;

    let max: u32 = fs::read_to_string(&max_brightness_file)
        .map_err(|e| VasakError::Io(e))?
        .trim()
        .parse()
        .map_err(|e| VasakError::Parse(format!("Failed to parse max brightness: {}", e)))?;

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

/// Obtiene el brillo usando xrandr (X11 only)
fn get_brightness_xrandr() -> Result<BrightnessInfo> {
    let output = CommandExecutor::run(CMD_XRANDR, &["--verbose"])?;

    let current_brightness = output
        .lines()
        .find(|line| line.contains("Brightness:"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<f32>().ok())
        .ok_or_else(|| VasakError::NotFound("Could not find current brightness".to_string()))?;

    let current = (current_brightness * 100.0) as u32;

    Ok(BrightnessInfo {
        current,
        max: 100,
        min: 0,
    })
}

/// Obtiene la información actual del brillo del sistema
pub fn get_brightness() -> Result<BrightnessInfo> {
    let method = get_brightness_method()?;
    
    match method {
        BrightnessMethod::Brightnessctl => get_brightness_brightnessctl(),
        BrightnessMethod::Sysfs(device_path) => get_brightness_sysfs(&device_path),
        BrightnessMethod::Xrandr => get_brightness_xrandr(),
    }
}

/// Establece el brillo usando brightnessctl
fn set_brightness_brightnessctl(brightness: u32) -> Result<()> {
    let brightness_str = format!("{}%", brightness);
    CommandExecutor::run(CMD_BRIGHTNESSCTL, &["set", &brightness_str])?;
    Ok(())
}

/// Establece el brillo usando sysfs
fn set_brightness_sysfs(device_path: &Path, brightness: u32) -> Result<()> {
    let brightness_file = device_path.join("brightness");
    let max_brightness_file = device_path.join("max_brightness");

    let max: u32 = fs::read_to_string(&max_brightness_file)
        .map_err(|e| VasakError::Io(e))?
        .trim()
        .parse()
        .map_err(|e| VasakError::Parse(format!("Failed to parse max brightness: {}", e)))?;

    // Convertir porcentaje a valor absoluto
    let absolute_value = ((brightness as f32 / 100.0) * max as f32) as u32;

    fs::write(&brightness_file, absolute_value.to_string())
        .map_err(|e| VasakError::Brightness(
            format!("Cannot write brightness (check permissions): {}", e)
        ))?;

    Ok(())
}

/// Establece el brillo usando xrandr (X11 only)
fn set_brightness_xrandr(brightness: u32) -> Result<()> {
    let brightness_value = (brightness as f32 / 100.0).to_string();

    let output = CommandExecutor::run(CMD_XRANDR, &["--listmonitors"])?;

    let monitors: Vec<&str> = output
        .lines()
        .skip(1)
        .filter_map(|line| line.split_whitespace().last())
        .collect();

    for monitor in monitors {
        CommandExecutor::run(CMD_XRANDR, &["--output", monitor, "--brightness", &brightness_value])?;
    }

    Ok(())
}

/// Establece el brillo del sistema
pub fn set_brightness(brightness: u32) -> Result<()> {
    let method = get_brightness_method()?;
    
    match method {
        BrightnessMethod::Brightnessctl => set_brightness_brightnessctl(brightness),
        BrightnessMethod::Sysfs(device_path) => set_brightness_sysfs(&device_path, brightness),
        BrightnessMethod::Xrandr => set_brightness_xrandr(brightness),
    }
}
