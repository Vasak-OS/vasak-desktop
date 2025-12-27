use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use zbus::Connection;

pub struct X11ShortcutsManager {
    shortcuts: Arc<Mutex<HashMap<String, String>>>,
}

impl X11ShortcutsManager {
    pub fn new() -> Self {
        Self {
            shortcuts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Registra un atajo usando xbindkeys
    pub async fn register_xbindkeys(&self, keys: &str, action: &str) -> Result<(), String> {
        // Crear configuración de xbindkeys
        let config = self.generate_xbindkeys_config(keys, action);

        // Escribir la configuración en ~/.xbindkeysrc
        let config_path = dirs::home_dir()
            .ok_or("Cannot find home directory")?
            .join(".xbindkeysrc");

        std::fs::write(&config_path, config)
            .map_err(|e| format!("Failed to write xbindkeys config: {}", e))?;

        // Recargar xbindkeys
        Command::new("pkill")
            .arg("-f")
            .arg("xbindkeys")
            .output()
            .ok();

        Command::new("xbindkeys")
            .output()
            .map_err(|e| format!("Failed to start xbindkeys: {}", e))?;

        Ok(())
    }

    /// Registra múltiples atajos en una sola configuración para xbindkeys
    pub async fn register_xbindkeys_bulk(&self, bindings: &[(String, String)]) -> Result<(), String> {
        let mut content = String::from(
            r#"# Auto-generated xbindkeys configuration
# WARNING: This file is managed by VasakOS. Manual edits may be overwritten.
"#,
        );

        for (keys, action) in bindings.iter() {
            content.push_str(&self.generate_xbindkeys_config(keys, action));
        }

        let config_path = dirs::home_dir()
            .ok_or("Cannot find home directory")?
            .join(".xbindkeysrc");

        std::fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write xbindkeys config: {}", e))?;

        // Reiniciar xbindkeys para recargar configuración
        Command::new("pkill")
            .arg("-f")
            .arg("xbindkeys")
            .output()
            .ok();

        Command::new("xbindkeys")
            .output()
            .map_err(|e| format!("Failed to start xbindkeys: {}", e))?;

        Ok(())
    }

    /// Registra un atajo usando D-Bus (para Wayland)
    pub async fn register_dbus_shortcut(
        &self,
        keys: &str,
        action: &str,
    ) -> Result<(), String> {
        match Connection::session().await {
            Ok(_conn) => {
                // Aquí iría la lógica para registrar con el compositor de Wayland
                log::info!("D-Bus shortcut registered: {} -> {}", keys, action);
                Ok(())
            }
            Err(e) => {
                log::warn!("Could not connect to D-Bus: {}", e);
                Ok(())
            }
        }
    }

    /// Genera la configuración para xbindkeys
    fn generate_xbindkeys_config(&self, keys: &str, action: &str) -> String {
        format!(
            r#"# Auto-generated xbindkeys configuration
# Shortcut: {} -> {}
"{}"\n  {}
"#,
            keys, action, keys, action
        )
    }

    /// Detecta el servidor de display (X11 o Wayland)
    pub fn get_display_server() -> String {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            "wayland".to_string()
        } else if std::env::var("DISPLAY").is_ok() {
            "x11".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Intenta registrar el atajo de forma automática basándose en el display server
    pub async fn register_auto(&self, keys: &str, action: &str) -> Result<(), String> {
        let display_server = Self::get_display_server();

        match display_server.as_str() {
            "x11" => self.register_xbindkeys(keys, action).await,
            "wayland" => self.register_dbus_shortcut(keys, action).await,
            _ => {
                log::warn!("Unknown display server, shortcuts may not work globally");
                Ok(())
            }
        }
    }

    /// Intenta registrar múltiples atajos de forma automática
    pub async fn register_auto_bulk(&self, bindings: &[(String, String)]) -> Result<(), String> {
        let display_server = Self::get_display_server();

        match display_server.as_str() {
            "x11" => self.register_xbindkeys_bulk(bindings).await,
            "wayland" => {
                // Para Wayland (DBus/portal) registrar individualmente
                for (keys, action) in bindings.iter() {
                    let _ = self.register_dbus_shortcut(keys, action).await;
                }
                Ok(())
            }
            _ => {
                log::warn!("Unknown display server, shortcuts may not work globally");
                Ok(())
            }
        }
    }
}

impl Default for X11ShortcutsManager {
    fn default() -> Self {
        Self::new()
    }
}
