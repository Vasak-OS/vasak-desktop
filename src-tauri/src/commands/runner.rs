use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::Command;

#[tauri::command]
pub async fn open_app(path: &str) -> Result<(), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        if let Ok(line) = line {
            if line.starts_with("Exec=") {
                let cmd = line.trim_start_matches("Exec=");
                let cmd = cmd.split_whitespace().next().unwrap_or(cmd);

                Command::new(cmd).spawn().map_err(|e| e.to_string())?;
            }
        }
    }

    Err("No se encontró el comando ejecutable".to_string())
}
