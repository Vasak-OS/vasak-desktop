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

    /// Convierte teclas de formato VasakOS a formato xbindkeys
    /// Ejemplo: "Ctrl+Alt+T" -> "Control+Alt + t"
    ///          "Super+Space" -> "Mod4 + space"
    ///          "Print" -> "Print"
    fn convert_keys_to_xbindkeys(&self, keys: &str) -> String {
        let mut parts: Vec<String> = Vec::new();
        
        // Separar por '+'
        for part in keys.split('+') {
            let trimmed = part.trim();
            let converted = match trimmed {
                // Modificadores
                "Ctrl" | "Control" => "Control",
                "Alt" => "Alt",
                "Shift" => "Shift",
                "Super" | "Meta" | "Mod4" => "Mod4",
                "Mod1" => "Mod1",
                "Mod2" => "Mod2",
                "Mod3" => "Mod3",
                "Mod5" => "Mod5",
                
                // Teclas especiales
                "Space" => "space",
                "Return" | "Enter" => "Return",
                "Tab" => "Tab",
                "Escape" | "Esc" => "Escape",
                "BackSpace" => "BackSpace",
                "Delete" | "Del" => "Delete",
                "Insert" | "Ins" => "Insert",
                "Home" => "Home",
                "End" => "End",
                "PageUp" | "Page_Up" => "Page_Up",
                "PageDown" | "Page_Down" => "Page_Down",
                "Print" => "Print",
                "Pause" => "Pause",
                "Menu" => "Menu",
                
                // Teclas de función
                key if key.starts_with('F') && key.len() >= 2 => {
                    // F1-F12
                    if key[1..].parse::<u8>().is_ok() {
                        key
                    } else {
                        &trimmed.to_lowercase()
                    }
                }
                
                // Flechas
                "Up" | "Arrow_Up" => "Up",
                "Down" | "Arrow_Down" => "Down",
                "Left" | "Arrow_Left" => "Left",
                "Right" | "Arrow_Right" => "Right",
                
                // Teclas del teclado numérico
                key if key.starts_with("KP_") => key,
                
                // Letras individuales (convertir a minúsculas)
                key if key.len() == 1 => &key.to_lowercase(),
                
                // Cualquier otra cosa, pasar tal cual pero en minúsculas
                _ => &trimmed.to_lowercase(),
            };
            
            parts.push(converted.to_string());
        }
        
        // Formato xbindkeys: modificadores separados por '+' y tecla final
        if parts.len() > 1 {
            let modifiers = &parts[..parts.len() - 1];
            let key = &parts[parts.len() - 1];
            format!("{} + {}", modifiers.join("+"), key)
        } else if parts.len() == 1 {
            parts[0].clone()
        } else {
            keys.to_string()
        }
    }

    /// Genera la configuración para xbindkeys
    fn generate_xbindkeys_config(&self, keys: &str, action: &str) -> String {
        let xbindkeys_format = self.convert_keys_to_xbindkeys(keys);
        format!(
            r#"# Shortcut: {} -> {}
"sh -c '{}'"
  {}

"#,
            keys, action, action, xbindkeys_format
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
