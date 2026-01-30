use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use chrono::Local;
use once_cell::sync::Lazy;

/// Nivel de log
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Fuente del log
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSource {
    Rust,
    JavaScript,
}

impl LogSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogSource::Rust => "RUST",
            LogSource::JavaScript => "JS",
        }
    }
}

/// Logger global
pub static LOGGER: Lazy<Mutex<VasakLogger>> = Lazy::new(|| {
    let logger = VasakLogger::new();
    Mutex::new(logger)
});

/// Logger principal de Vasak Desktop
pub struct VasakLogger {
    log_file: Option<File>,
    log_path: PathBuf,
    is_dev_mode: bool,
}

impl VasakLogger {
    /// Crea una nueva instancia del logger
    pub fn new() -> Self {
        let is_dev_mode = cfg!(debug_assertions);
        
        // Determinar la ruta del archivo de log
        let log_path = Self::get_log_path();
        
        // Crear el directorio si no existe
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        
        // Abrir o crear el archivo de log
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok();
        
        if log_file.is_none() {
            eprintln!("⚠️ No se pudo crear el archivo de log en: {:?}", log_path);
        }
        
        let mut logger = Self {
            log_file,
            log_path,
            is_dev_mode,
        };
        
        // Escribir encabezado de sesión
        logger.log_session_start();
        
        logger
    }
    
    /// Obtiene la ruta del archivo de log
    fn get_log_path() -> PathBuf {
        // Usar XDG_DATA_HOME o ~/.local/share como base
        let base_dir = dirs::data_local_dir()
            .unwrap_or_else(|| {
                let home = dirs::home_dir().expect("No se pudo obtener el directorio home");
                home.join(".local/share")
            });
        
        let log_dir = base_dir.join("vasak-desktop").join("logs");
        
        // Nombre del archivo con fecha
        let date = Local::now().format("%Y-%m-%d");
        log_dir.join(format!("vasak-desktop-{}.log", date))
    }
    
    /// Escribe el encabezado de inicio de sesión
    fn log_session_start(&mut self) {
        let mode = if self.is_dev_mode { "DESARROLLO" } else { "PRODUCCIÓN" };
        let separator = "=".repeat(80);
        
        self.write_to_file(&format!("\n{}\n", separator));
        self.write_to_file(&format!("Nueva sesión iniciada: {}\n", Local::now().format("%Y-%m-%d %H:%M:%S")));
        self.write_to_file(&format!("Modo: {}\n", mode));
        self.write_to_file(&format!("Archivo de log: {:?}\n", self.log_path));
        self.write_to_file(&format!("{}\n\n", separator));
    }
    
    /// Escribe un mensaje en el archivo
    fn write_to_file(&mut self, message: &str) {
        if let Some(ref mut file) = self.log_file {
            let _ = file.write_all(message.as_bytes());
            let _ = file.flush();
        }
    }
    
    /// Registra un mensaje
    pub fn log(&mut self, level: LogLevel, source: LogSource, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let formatted_message = format!(
            "[{}] [{:>8}] [{:>4}] {}\n",
            timestamp,
            level.as_str(),
            source.as_str(),
            message
        );
        
        // Escribir al archivo
        self.write_to_file(&formatted_message);
        
        // En modo desarrollo, también imprimir en consola
        if self.is_dev_mode {
            match level {
                LogLevel::Error => eprint!("{}", formatted_message),
                LogLevel::Warning => eprint!("{}", formatted_message),
                LogLevel::Info | LogLevel::Debug => print!("{}", formatted_message),
            }
        }
    }
    
    /// Obtiene la ruta actual del log
    pub fn get_current_log_path(&self) -> PathBuf {
        self.log_path.clone()
    }
}

/// Funciones de conveniencia para logging desde Rust
#[allow(dead_code)]
pub fn log_debug(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Debug, LogSource::Rust, message);
    }
}

#[allow(dead_code)]
pub fn log_info(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Info, LogSource::Rust, message);
    }
}

#[allow(dead_code)]
pub fn log_warning(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Warning, LogSource::Rust, message);
    }
}

#[allow(dead_code)]
pub fn log_error(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Error, LogSource::Rust, message);
    }
}

/// Log desde JavaScript
pub fn log_from_js(level: &str, message: &str) {
    let log_level = match level.to_uppercase().as_str() {
        "DEBUG" => LogLevel::Debug,
        "INFO" => LogLevel::Info,
        "WARNING" | "WARN" => LogLevel::Warning,
        "ERROR" => LogLevel::Error,
        _ => LogLevel::Info,
    };
    
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(log_level, LogSource::JavaScript, message);
    }
}

/// Obtiene la ruta del archivo de log actual
pub fn get_log_file_path() -> String {
    if let Ok(logger) = LOGGER.lock() {
        logger.get_current_log_path().to_string_lossy().to_string()
    } else {
        String::from("Error: No se pudo acceder al logger")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_creation() {
        let logger = VasakLogger::new();
        assert!(logger.log_path.to_string_lossy().contains("vasak-desktop"));
    }

    #[test]
    fn test_log_levels() {
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warning.as_str(), "WARNING");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
    }

    #[test]
    fn test_log_sources() {
        assert_eq!(LogSource::Rust.as_str(), "RUST");
        assert_eq!(LogSource::JavaScript.as_str(), "JS");
    }
}
