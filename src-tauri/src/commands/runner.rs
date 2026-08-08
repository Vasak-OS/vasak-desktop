use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::Command;
use crate::logger::{log_info, log_error};

fn parse_exec_line(exec_line: &str) -> Result<(String, Vec<String>), String> {
    let parts = shlex::split(exec_line).ok_or_else(|| "No se pudo parsear Exec".to_string())?;

    if parts.is_empty() {
        return Err("Exec vacío".to_string());
    }

    let command = parts[0].clone();
    let args = parts
        .into_iter()
        .skip(1)
        .filter(|arg| !(arg.starts_with('%') && arg.len() == 2))
        .map(|arg| arg.replace("%%", "%"))
        .collect();

    Ok((command, args))
}

#[allow(clippy::lines_filter_map_ok)]
#[tauri::command]
pub async fn open_app(path: &str) -> Result<(), String> {
    log_info(&format!("Abriendo aplicación desde: {}", path));
    let file = File::open(path).map_err(|e| {
        log_error(&format!("Error al abrir archivo .desktop {}: {}", path, e));
        e.to_string()
    })?;
    let reader = BufReader::new(file);

    for line in reader.lines().flatten() {
        if line.starts_with("Exec=") {
            let exec_line = line.trim_start_matches("Exec=");
            let (cmd, args) = parse_exec_line(exec_line).map_err(|e| {
                log_error(&format!("Error parseando Exec en {}: {}", path, e));
                e
            })?;

            log_info(&format!("Ejecutando comando: {} {:?}", cmd, args));
            Command::new(&cmd).args(&args).spawn().map_err(|e| {
                log_error(&format!("Error al ejecutar comando {} {:?}: {}", cmd, args, e));
                e.to_string()
            })?;

            return Ok(());
        }
    }

    log_error(&format!("No se encontró comando ejecutable en: {}", path));
    Err("No se encontró el comando ejecutable".to_string())
}

/// Binary shipped by the vasak-settings package.
const SETTINGS_BINARY: &str = "vasak-settings";

/// Launches the VasakOS settings application.
///
/// The menu's gear button invoked `open_configuration_window`, a command that
/// was never implemented on the Rust side: the call rejected, the error was
/// swallowed by the caller's catch, and the button silently did nothing. The
/// shell has no settings UI of its own — vasak-settings is a separate app — so
/// the button's job is simply to launch it.
pub fn spawn_settings() -> Result<(), String> {
    log_info("Abriendo la aplicación de configuración");

    Command::new(SETTINGS_BINARY).spawn().map_err(|error| {
        let message = format!(
            "No se pudo abrir {}: {}. ¿Está instalado el paquete vasak-settings?",
            SETTINGS_BINARY, error
        );
        log_error(&message);
        message
    })?;

    Ok(())
}

#[tauri::command]
pub async fn open_settings() -> Result<(), String> {
    spawn_settings()
}
