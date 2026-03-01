use crate::utils::shortcuts::shortcuts::{Shortcut, ShortcutsManager};
use crate::utils::shortcuts::platform_shortcuts::X11ShortcutsManager;
use std::process::Command;
use tauri::{AppHandle, Emitter};
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct ShortcutAction {
    pub action_type: String,
    pub data: Option<String>,
}

pub struct GlobalShortcutsHandler {
    manager: ShortcutsManager,
}

impl GlobalShortcutsHandler {
    pub fn new() -> Self {
        Self {
            manager: ShortcutsManager::new(),
        }
    }

    fn vasak_dbus_command(&self, method: &str) -> String {
        format!(
            "dbus-send --session --dest=org.vasak.os.Desktop --type=method_call / org.vasak.os.Desktop.{}",
            method
        )
    }

    /// Registra todos los atajos globales en el sistema
    pub async fn register_all(&self, _app: AppHandle) -> Result<(), String> {
        let shortcuts = self.manager.get_shortcuts();

        // Construir lista de (keys, action) para registro a nivel sistema
        let mut bindings: Vec<(String, String)> = Vec::new();
        for s in shortcuts.iter() {
            if let Some(action) = self.get_action_for_shortcut(&s.id) {
                bindings.push((s.keys.clone(), action));
            }
        }

        // Registrar en el sistema (X11/Wayland) en modo bulk
        let mgr = X11ShortcutsManager::new();
        let _ = mgr.register_auto_bulk(&bindings).await;

        Ok(())
    }

    /// Registra un atajo individual en el sistema
    #[allow(dead_code)]
    fn register_shortcut(&self, shortcut: &Shortcut) -> Result<(), String> {
        // Convertir las teclas a un formato que entienda xdotool
        let xdotool_keys = self.convert_to_xdotool_format(&shortcut.keys)?;

        // Usar xbindkeys o similar para registrar el atajo
        // Por ahora usaremos un enfoque simple con un script que se ejecuta en background
        self.setup_shortcut_binding(&xdotool_keys, &shortcut.id)
            .ok();

        Ok(())
    }

    /// Convierte el formato "Ctrl+K" a "ctrl+k" (xdotool)
    #[allow(dead_code)]
    fn convert_to_xdotool_format(&self, keys: &str) -> Result<String, String> {
        // Ejemplo: "Ctrl+K" -> "ctrl+k"
        Ok(keys.to_lowercase())
    }

    /// Configura el binding del atajo
    #[allow(dead_code)]
    fn setup_shortcut_binding(&self, xdotool_keys: &str, shortcut_id: &str) -> Result<(), String> {
        // Crear un comando que se ejecuta cuando se presiona el atajo
        let action = self.get_action_for_shortcut(shortcut_id);

        if let Some(action) = action {
            // En un ambiente real, usarías xbindkeys, xhotkey o el API de Wayland
            // Por ahora, registramos la intención
            log::info!(
                "Shortcut registered: {} -> Action: {}",
                xdotool_keys,
                action
            );
        }

        Ok(())
    }

    /// Obtiene la acción a ejecutar para un atajo
    pub fn get_action_for_shortcut(&self, shortcut_id: &str) -> Option<String> {
        match shortcut_id {
            // VasakOS shortcuts
            "vasak_search" => Some(self.vasak_dbus_command("OpenSearch")),
            "vasak_menu" => Some(self.vasak_dbus_command("OpenMenu")),
            "vasak_control_center" => Some(self.vasak_dbus_command("OpenControlCenter")),
            "vasak_config" => Some(self.vasak_dbus_command("OpenConfigApp")),

            // System shortcuts - comandos más estándar y seguros
            "system_terminal" => {
                // Intenta gnome-terminal primero, luego tienes alternativas
                Some("which gnome-terminal >/dev/null 2>&1 && exec gnome-terminal || which xterm >/dev/null 2>&1 && exec xterm || which konsole >/dev/null 2>&1 && exec konsole".to_string())
            }
            "system_file_manager" => {
                // Intenta nautilus primero
                Some("which nautilus >/dev/null 2>&1 && exec nautilus || which dolphin >/dev/null 2>&1 && exec dolphin || which thunar >/dev/null 2>&1 && exec thunar".to_string())
            }
            "system_lock" => {
                // Usa loginctl que es más portable
                Some("loginctl lock-session".to_string())
            }
            "system_screenshot" => {
                // Intenta herramientas de captura estándar
                Some("which gnome-screenshot >/dev/null 2>&1 && exec gnome-screenshot || which scrot >/dev/null 2>&1 && exec scrot /tmp/screenshot-$RANDOM.png".to_string())
            }

            // Custom shortcuts
            id if id.starts_with("custom_") => {
                if let Some(shortcut) = self.manager.get_shortcut(id) {
                    shortcut.command
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    /// Retorna la acción a ejecutar sin ejecutarla
    pub fn get_action_info(&self, shortcut_id: &str) -> Result<ShortcutAction, String> {
        match shortcut_id {
            // VasakOS shortcuts - retornar el tipo de acción
            "vasak_search" => Ok(ShortcutAction {
                action_type: "vasak_command".to_string(),
                data: Some("toggle_search".to_string()),
            }),
            "vasak_menu" => Ok(ShortcutAction {
                action_type: "vasak_command".to_string(),
                data: Some("toggle_menu".to_string()),
            }),
            "vasak_control_center" => Ok(ShortcutAction {
                action_type: "vasak_command".to_string(),
                data: Some("toggle_control_center".to_string()),
            }),
            "vasak_config" => Ok(ShortcutAction {
                action_type: "vasak_command".to_string(),
                data: Some("open_configuration_window".to_string()),
            }),

            // System shortcuts - ejecutar directamente
            _ => {
                if let Some(action) = self.get_action_for_shortcut(shortcut_id) {
                    Ok(ShortcutAction {
                        action_type: "system_command".to_string(),
                        data: Some(action),
                    })
                } else {
                    Err(format!("No action found for shortcut: {}", shortcut_id))
                }
            }
        }
    }

    /// Ejecuta la acción de un atajo
    pub fn execute_action(&self, shortcut_id: &str, app: AppHandle) -> Result<(), String> {
        let action_info = self.get_action_info(shortcut_id)?;

        match action_info.action_type.as_str() {
            "vasak_command" => {
                let dbus_method = match shortcut_id {
                    "vasak_search" => Some("OpenSearch"),
                    "vasak_menu" => Some("OpenMenu"),
                    "vasak_control_center" => Some("OpenControlCenter"),
                    "vasak_config" => Some("OpenConfigApp"),
                    _ => None,
                };

                if let Some(method) = dbus_method {
                    let command = self.vasak_dbus_command(method);
                    if let Err(err) = self.execute_system_command(&command) {
                        log::warn!(
                            "Fallo al ejecutar D-Bus para {}: {}. Se usa fallback interno.",
                            shortcut_id, err
                        );
                    }
                }

                if let Some(command) = action_info.data {
                    let event_name = format!("shortcut:{}", command);
                    let _ = app.emit_to(
                        tauri::EventTarget::Any,
                        &event_name,
                        shortcut_id,
                    );
                }
            }
            "system_command" => {
                if let Some(command) = action_info.data {
                    self.execute_system_command(&command)?;
                }
            }
            _ => {
                return Err(format!(
                    "Unknown action type: {}",
                    action_info.action_type
                ))
            }
        }

        Ok(())
    }

    /// Ejecuta un comando del sistema de forma segura
    fn execute_system_command(&self, command: &str) -> Result<(), String> {
        log::info!("Executing system command: {}", command);

        // Usar sh -c para permitir que shell resuelva PATH, variables y comandos condicionales
        match Command::new("sh")
            .arg("-c")
            .arg(command)
            .spawn()
        {
            Ok(_) => {
                log::info!("Command spawned successfully: {}", command);
                Ok(())
            }
            Err(e) => {
                let error_msg = match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        format!("No se puede ejecutar el comando. ¿Está instalado 'sh'? Error: {}", e)
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        format!("Permiso denegado para ejecutar el comando. Error: {}", e)
                    }
                    _ => {
                        format!("Error al ejecutar comando '{}': {}", command, e)
                    }
                };
                log::error!("{}", error_msg);
                Err(error_msg)
            }
        }
    }
}

impl Default for GlobalShortcutsHandler {
    fn default() -> Self {
        Self::new()
    }
}
