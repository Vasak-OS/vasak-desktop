use std::path::{Path, PathBuf};
use std::fs;
use std::sync::OnceLock;
use crate::constants::{CMD_BRIGHTNESSCTL, CMD_BUSCTL, BACKLIGHT_PATH};
use crate::error::{Result, VasakError};
use crate::logger::{log_info, log_error, log_debug};
use crate::structs::BrightnessInfo;
use crate::utils::CommandExecutor;

/// Método de control de brillo disponible
#[derive(Clone)]
enum BrightnessMethod {
    Brightnessctl,
    Logind(PathBuf),
    Sysfs(PathBuf),
}

/// Cache global del método de brillo detectado
static BRIGHTNESS_METHOD: OnceLock<BrightnessMethod> = OnceLock::new();

/// Obtiene el método de brillo, detectándolo solo la primera vez
fn get_brightness_method() -> Result<BrightnessMethod> {
    if let Some(method) = BRIGHTNESS_METHOD.get() {
        return Ok(method.clone());
    }

    let method = detect_brightness_method()?;

    let _ = BRIGHTNESS_METHOD.set(method.clone());

    Ok(method)
}

/// Detecta el mejor método de control de brillo disponible
fn detect_brightness_method() -> Result<BrightnessMethod> {
    log_info("Detectando método de control de brillo");

    // 1. brightnessctl (no requiere contraseña con udev rules)
    if CommandExecutor::run_silent(CMD_BRIGHTNESSCTL, &["get"]) {
        log_info("Usando brightnessctl para control de brillo");
        return Ok(BrightnessMethod::Brightnessctl);
    }

    // 2. logind D-Bus via busctl (no requiere contraseña)
    if let Ok(device_path) = find_backlight_device() {
        if CommandExecutor::run_silent(CMD_BUSCTL, &["--version"]) {
            log_info(&format!("Usando logind (busctl) para control de brillo: {:?}", device_path));
            return Ok(BrightnessMethod::Logind(device_path));
        }
    }

    // 3. sysfs directo (puede fallar si no hay permisos)
    if let Ok(device_path) = find_backlight_device() {
        let test_path = device_path.join("brightness");
        if fs::read_to_string(&test_path).is_ok() {
            log_info(&format!("Usando sysfs para control de brillo: {:?}", device_path));
            return Ok(BrightnessMethod::Sysfs(device_path));
        }
    }

    log_error("No se encontró método de control de brillo disponible");
    Err(VasakError::Brightness(
        "No brightness control method available. Install 'brightnessctl' (recommended) or add user to the 'video' group".to_string()
    ))
}

/// Encuentra el primer dispositivo de backlight disponible
fn find_backlight_device() -> Result<PathBuf> {
    if !Path::new(BACKLIGHT_PATH).exists() {
        return Err(VasakError::NotFound("No backlight devices found".to_string()));
    }

    let entries = fs::read_dir(BACKLIGHT_PATH)
        .map_err(VasakError::Io)?;

    for entry in entries {
        let entry = entry.map_err(VasakError::Io)?;
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
        .map_err(VasakError::Io)?
        .trim()
        .parse()
        .map_err(|e| VasakError::Parse(format!("Failed to parse brightness: {}", e)))?;

    let max: u32 = fs::read_to_string(&max_brightness_file)
        .map_err(VasakError::Io)?
        .trim()
        .parse()
        .map_err(|e| VasakError::Parse(format!("Failed to parse max brightness: {}", e)))?;

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

/// Obtiene la información actual del brillo del sistema
pub fn get_brightness() -> Result<BrightnessInfo> {
    log_debug("Obteniendo información de brillo del sistema");
    let method = get_brightness_method()?;

    let result = match method {
        BrightnessMethod::Brightnessctl => get_brightness_brightnessctl(),
        BrightnessMethod::Logind(ref device_path) | BrightnessMethod::Sysfs(ref device_path) => get_brightness_sysfs(device_path),
    };

    if let Ok(ref info) = result {
        log_debug(&format!("Brillo actual: {}%", info.current));
    }
    result
}

/// Establece el brillo usando brightnessctl
fn set_brightness_brightnessctl(brightness: u32) -> Result<()> {
    let brightness_str = format!("{}%", brightness);
    CommandExecutor::run(CMD_BRIGHTNESSCTL, &["set", &brightness_str])?;
    Ok(())
}

/// Establece el brillo usando logind D-Bus (busctl)
fn set_brightness_logind(device_path: &Path, brightness: u32) -> Result<()> {
    let max_brightness_file = device_path.join("max_brightness");
    let device_name = device_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| VasakError::Brightness("Invalid device name".to_string()))?;

    let max: u32 = fs::read_to_string(&max_brightness_file)
        .map_err(VasakError::Io)?
        .trim()
        .parse()
        .map_err(|e| VasakError::Parse(format!("Failed to parse max brightness: {}", e)))?;

    let absolute_value = ((brightness as f32 / 100.0) * max as f32) as u32;

    CommandExecutor::run(CMD_BUSCTL, &[
        "call",
        "org.freedesktop.login1",
        "/org/freedesktop/login1/session/auto",
        "org.freedesktop.login1.Session",
        "SetBrightness",
        "ssu",
        "backlight",
        device_name,
        &absolute_value.to_string(),
    ])?;

    Ok(())
}

/// Establece el brillo usando sysfs
fn set_brightness_sysfs(device_path: &Path, brightness: u32) -> Result<()> {
    let brightness_file = device_path.join("brightness");
    let max_brightness_file = device_path.join("max_brightness");

    let max: u32 = fs::read_to_string(&max_brightness_file)
        .map_err(VasakError::Io)?
        .trim()
        .parse()
        .map_err(|e| VasakError::Parse(format!("Failed to parse max brightness: {}", e)))?;

    let absolute_value = ((brightness as f32 / 100.0) * max as f32) as u32;

    fs::write(&brightness_file, absolute_value.to_string())
        .map_err(|e| VasakError::Brightness(
            format!("Cannot write brightness (check permissions): {}", e)
        ))?;

    Ok(())
}

/// Establece el brillo del sistema
pub fn set_brightness(brightness: u32) -> Result<()> {
    log_debug(&format!("Estableciendo brillo del sistema a: {}%", brightness));
    let method = get_brightness_method()?;

    let result = match method {
        BrightnessMethod::Brightnessctl => set_brightness_brightnessctl(brightness),
        BrightnessMethod::Logind(ref device_path) => set_brightness_logind(device_path, brightness),
        BrightnessMethod::Sysfs(ref device_path) => set_brightness_sysfs(device_path, brightness),
    };

    if result.is_ok() {
        log_debug(&format!("Brillo establecido correctamente a: {}%", brightness));
    } else if let Err(ref e) = result {
        log_error(&format!("Error al establecer brillo: {}", e));
    }
    result
}
