use crate::logger;

/// Comando para registrar mensajes desde JavaScript
#[tauri::command]
pub fn log_from_frontend(level: String, message: String) {
    logger::log_from_js(&level, &message);
}

/// Comando para obtener la ruta del archivo de log actual
#[tauri::command]
pub fn get_log_file_path() -> String {
    logger::get_log_file_path()
}

/// Comando para leer el contenido del archivo de log
#[tauri::command]
pub fn read_log_file() -> Result<String, String> {
    use std::fs;
    
    let log_path = logger::get_log_file_path();
    
    fs::read_to_string(&log_path)
        .map_err(|e| format!("Error al leer el archivo de log: {}", e))
}

/// Comando para obtener las últimas N líneas del log
#[tauri::command]
pub fn get_last_log_lines(lines: usize) -> Result<Vec<String>, String> {
    use std::fs;
    
    let log_path = logger::get_log_file_path();
    
    let content = fs::read_to_string(&log_path)
        .map_err(|e| format!("Error al leer el archivo de log: {}", e))?;
    
    let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let start = if all_lines.len() > lines {
        all_lines.len() - lines
    } else {
        0
    };
    
    Ok(all_lines[start..].to_vec())
}
