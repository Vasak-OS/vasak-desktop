use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use crate::error::{Result, VasakError};
use crate::logger::{log_debug, log_error};
use crate::constants::DEFAULT_COMMAND_TIMEOUT_SECS;

/// Ejecutor de comandos del sistema con soporte para timeouts
pub struct CommandExecutor;

impl CommandExecutor {
    /// Ejecuta un comando con el timeout por defecto (3 segundos)
    pub fn run(cmd: &str, args: &[&str]) -> Result<String> {
        Self::run_with_timeout(cmd, args, Duration::from_secs(DEFAULT_COMMAND_TIMEOUT_SECS))
    }
    
    /// Ejecuta un comando con un timeout personalizado
    pub fn run_with_timeout(cmd: &str, args: &[&str], timeout: Duration) -> Result<String> {
        log_debug(&format!("Ejecutando comando: {} {:?}", cmd, args));
        let cmd_owned = cmd.to_string();
        let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        
        let (tx, rx) = mpsc::channel();
        
        thread::spawn(move || {
            let output = Command::new(&cmd_owned)
                .args(&args_owned)
                .output();
            let _ = tx.send(output);
        });
        
        let output = rx.recv_timeout(timeout)
            .map_err(|_| {
                log_error(&format!("Timeout ejecutando comando: {} (timeout: {:?})", cmd, timeout));
                VasakError::CommandTimeout { timeout }
            })?
            .map_err(|e| {
                log_error(&format!("Error al ejecutar comando {}: {}", cmd, e));
                VasakError::Command(format!("Failed to execute {}: {}", cmd, e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log_error(&format!("Comando {} falló: {}", cmd, stderr));
            return Err(VasakError::Command(format!(
                "Command {} failed: {}",
                cmd,
                stderr
            )));
        }

        log_debug(&format!("Comando {} ejecutado exitosamente", cmd));
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
    
    /// Ejecuta un comando y retorna true si tiene éxito, false en caso contrario
    pub fn run_silent(cmd: &str, args: &[&str]) -> bool {
        Self::run(cmd, args).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_simple_command() {
        let result = CommandExecutor::run("echo", &["hello"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "hello");
    }

    #[test]
    fn test_run_with_timeout() {
        let result = CommandExecutor::run_with_timeout(
            "echo", 
            &["test"], 
            Duration::from_secs(1)
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_silent() {
        assert!(CommandExecutor::run_silent("echo", &["test"]));
        assert!(!CommandExecutor::run_silent("nonexistent_command_xyz", &[]));
    }
}
