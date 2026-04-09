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
