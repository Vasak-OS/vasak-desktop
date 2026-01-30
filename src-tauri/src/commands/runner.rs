use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::Command;
use crate::logger::{log_info, log_error};

#[tauri::command]
pub async fn open_app(path: &str) -> Result<(), String> {
    log_info(&format!("Abriendo aplicación desde: {}", path));
    let file = File::open(path).map_err(|e| {
        log_error(&format!("Error al abrir archivo .desktop {}: {}", path, e));
        e.to_string()
    })?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        if let Ok(line) = line {
            if line.starts_with("Exec=") {
                let cmd = line.trim_start_matches("Exec=");
                let cmd = cmd.split_whitespace().next().unwrap_or(cmd);

                log_info(&format!("Ejecutando comando: {}", cmd));
                Command::new(cmd).spawn().map_err(|e| {
                    log_error(&format!("Error al ejecutar comando {}: {}", cmd, e));
                    e.to_string()
                })?;
            }
        }
    }

    log_error(&format!("No se encontró comando ejecutable en: {}", path));
    Err("No se encontró el comando ejecutable".to_string())
}
