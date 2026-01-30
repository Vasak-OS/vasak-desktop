use thiserror::Error;
use crate::logger;

/// Tipo de error principal para la aplicación Vasak Desktop
#[derive(Error, Debug)]
pub enum VasakError {
    #[error("Audio error: {0}")]
    Audio(String),
    
    #[error("Brightness error: {0}")]
    Brightness(String),
    
    #[error("DBus error: {0}")]
    DBus(#[from] zbus::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Command execution failed: {0}")]
    Command(String),
    
    #[error("Command timed out after {timeout:?}")]
    CommandTimeout { timeout: std::time::Duration },
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Lock poisoned: {0}")]
    LockPoisoned(&'static str),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

impl VasakError {
    /// Log el error automáticamente
    pub fn log(&self) {
        logger::log_error(&format!("VasakError: {}", self));
    }
}

/// Tipo Result personalizado para la aplicación
pub type Result<T> = std::result::Result<T, VasakError>;

impl From<VasakError> for String {
    fn from(err: VasakError) -> String {
        // Log el error cuando se convierte a String
        logger::log_error(&format!("Error convertido a String: {}", err));
        err.to_string()
    }
}
