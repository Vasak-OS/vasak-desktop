use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shortcut {
    pub id: String,
    pub name: String,
    pub description: String,
    pub keys: String,
    pub category: String, // "system", "vasak", "custom"
    pub editable: bool,
    pub command: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ShortcutsConfig {
    shortcuts: Vec<Shortcut>,
}

pub struct ShortcutsManager {
    config_path: PathBuf,
    shortcuts: Vec<Shortcut>,
}

impl ShortcutsManager {
    pub fn new() -> Self {
        let mut manager = Self {
            config_path: Self::get_config_path(),
            shortcuts: Vec::new(),
        };
        manager.load_or_create_default();
        manager
    }

    fn get_config_path() -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
        let vasak_dir = config_dir.join("vasak");
        if !vasak_dir.exists() {
            let _ = fs::create_dir_all(&vasak_dir);
        }
        vasak_dir.join("shortcuts.json")
    }

    fn load_or_create_default(&mut self) {
        if self.config_path.exists() {
            if let Ok(content) = fs::read_to_string(&self.config_path) {
                if let Ok(config) = serde_json::from_str::<ShortcutsConfig>(&content) {
                    self.shortcuts = config.shortcuts;
                    return;
                }
            }
        }

        // Create default shortcuts
        self.shortcuts = vec![
            // System shortcuts
            Shortcut {
                id: "system_terminal".to_string(),
                name: "Abrir Terminal".to_string(),
                description: "Abre una nueva ventana de terminal".to_string(),
                keys: "Ctrl+Alt+T".to_string(),
                category: "system".to_string(),
                editable: true,
                command: Some("gnome-terminal".to_string()),
            },
            Shortcut {
                id: "system_file_manager".to_string(),
                name: "Gestor de Archivos".to_string(),
                description: "Abre el gestor de archivos".to_string(),
                keys: "Super+E".to_string(),
                category: "system".to_string(),
                editable: true,
                command: Some("nautilus".to_string()),
            },
            Shortcut {
                id: "system_lock".to_string(),
                name: "Bloquear Pantalla".to_string(),
                description: "Bloquea la sesión actual".to_string(),
                keys: "Super+L".to_string(),
                category: "system".to_string(),
                editable: true,
                command: Some("loginctl lock-session".to_string()),
            },
            Shortcut {
                id: "system_screenshot".to_string(),
                name: "Captura de Pantalla".to_string(),
                description: "Toma una captura de toda la pantalla".to_string(),
                keys: "Print".to_string(),
                category: "system".to_string(),
                editable: true,
                command: Some("gnome-screenshot".to_string()),
            },
            // VasakOS shortcuts
            Shortcut {
                id: "vasak_search".to_string(),
                name: "Búsqueda Global".to_string(),
                description: "Abre la búsqueda global del sistema".to_string(),
                keys: "Super+Space".to_string(),
                category: "vasak".to_string(),
                editable: true,
                command: None,
            },
            Shortcut {
                id: "vasak_menu".to_string(),
                name: "Menú de Aplicaciones".to_string(),
                description: "Abre el menú principal de VasakOS".to_string(),
                keys: "Super".to_string(),
                category: "vasak".to_string(),
                editable: true,
                command: None,
            },
            Shortcut {
                id: "vasak_control_center".to_string(),
                name: "Centro de Control".to_string(),
                description: "Abre el centro de control del sistema".to_string(),
                keys: "Super+C".to_string(),
                category: "vasak".to_string(),
                editable: true,
                command: None,
            },
            Shortcut {
                id: "vasak_config".to_string(),
                name: "Configuración".to_string(),
                description: "Abre la aplicación de configuración".to_string(),
                keys: "Super+,".to_string(),
                category: "vasak".to_string(),
                editable: true,
                command: None,
            },
        ];

        self.save();
    }

    pub fn get_shortcuts(&self) -> Vec<Shortcut> {
        self.shortcuts.clone()
    }

    pub fn get_shortcut(&self, id: &str) -> Option<Shortcut> {
        self.shortcuts.iter().find(|s| s.id == id).cloned()
    }

    pub fn update_shortcut(&mut self, id: &str, keys: &str) -> Result<Shortcut, String> {
        let shortcut = self
            .shortcuts
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| format!("Shortcut '{}' not found", id))?;

        if !shortcut.editable {
            return Err("This shortcut cannot be edited".to_string());
        }

        shortcut.keys = keys.to_string();
        let result = shortcut.clone();
        self.save();
        Ok(result)
    }

    pub fn add_custom_shortcut(
        &mut self,
        name: String,
        description: String,
        keys: String,
        command: String,
    ) -> Result<Shortcut, String> {
        let id = format!("custom_{}", uuid::Uuid::new_v4());

        let shortcut = Shortcut {
            id: id.clone(),
            name,
            description,
            keys,
            category: "custom".to_string(),
            editable: true,
            command: Some(command),
        };

        self.shortcuts.push(shortcut.clone());
        self.save();
        Ok(shortcut)
    }

    pub fn delete_shortcut(&mut self, id: &str) -> Result<(), String> {
        let index = self
            .shortcuts
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| format!("Shortcut '{}' not found", id))?;

        let shortcut = &self.shortcuts[index];
        if shortcut.category != "custom" {
            return Err("Only custom shortcuts can be deleted".to_string());
        }

        self.shortcuts.remove(index);
        self.save();
        Ok(())
    }

    pub fn save(&self) {
        let config = ShortcutsConfig {
            shortcuts: self.shortcuts.clone(),
        };

        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(&self.config_path, json);
        }
    }
}

impl Default for ShortcutsManager {
    fn default() -> Self {
        Self::new()
    }
}
