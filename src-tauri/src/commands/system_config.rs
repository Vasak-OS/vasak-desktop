use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub border_radius: u32,
    pub primary_color: String,
    pub accent_color: String,
    pub dark_mode: bool,
    pub icon_pack: String,
    pub cursor_theme: String,
    pub gtk_theme: String,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            border_radius: 8,
            primary_color: "#0084FF".to_string(),
            accent_color: "#FF6B6B".to_string(),
            dark_mode: false,
            icon_pack: "Adwaita".to_string(),
            cursor_theme: "Adwaita".to_string(),
            gtk_theme: "Adwaita".to_string(),
        }
    }
}

/// Obtiene la configuración actual del sistema desde archivo
#[tauri::command]
pub async fn get_system_config() -> Result<SystemConfig, String> {
    let config_path = get_config_path()?;
    
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Error leyendo configuración: {}", e))?;
        
        serde_json::from_str(&content)
            .map_err(|e| format!("Error parseando configuración: {}", e))
    } else {
        Ok(SystemConfig::default())
    }
}

/// Obtiene el estado actual real del sistema desde gsettings
#[tauri::command]
pub async fn get_current_system_state() -> Result<SystemConfig, String> {
    let gtk_theme = get_current_gtk_theme().await.unwrap_or_else(|_| "Adwaita".to_string());
    let cursor_theme = get_current_cursor_theme().await.unwrap_or_else(|_| "Adwaita".to_string());
    let icon_pack = get_current_icon_pack().await.unwrap_or_else(|_| "Adwaita".to_string());
    let dark_mode = get_current_dark_mode().await.unwrap_or(false);
    
    Ok(SystemConfig {
        border_radius: 8,
        primary_color: "#0084FF".to_string(),
        accent_color: "#FF6B6B".to_string(),
        dark_mode,
        icon_pack,
        cursor_theme,
        gtk_theme,
    })
}

/// Obtiene el tema GTK actual desde gsettings
async fn get_current_gtk_theme() -> Result<String, String> {
    let output = Command::new("gsettings")
        .args(&["get", "org.gnome.desktop.interface", "gtk-theme"])
        .output()
        .map_err(|e| format!("Error obteniendo tema GTK: {}", e))?;
    
    let theme = String::from_utf8_lossy(&output.stdout)
        .trim()
        .trim_matches('\'')
        .to_string();
    
    Ok(theme)
}

/// Obtiene el cursor actual desde gsettings
async fn get_current_cursor_theme() -> Result<String, String> {
    let output = Command::new("gsettings")
        .args(&["get", "org.gnome.desktop.interface", "cursor-theme"])
        .output()
        .map_err(|e| format!("Error obteniendo cursor: {}", e))?;
    
    let cursor = String::from_utf8_lossy(&output.stdout)
        .trim()
        .trim_matches('\'')
        .to_string();
    
    Ok(cursor)
}

/// Obtiene el pack de iconos actual desde gsettings
async fn get_current_icon_pack() -> Result<String, String> {
    let output = Command::new("gsettings")
        .args(&["get", "org.gnome.desktop.interface", "icon-theme"])
        .output()
        .map_err(|e| format!("Error obteniendo pack de iconos: {}", e))?;
    
    let icons = String::from_utf8_lossy(&output.stdout)
        .trim()
        .trim_matches('\'')
        .to_string();
    
    Ok(icons)
}

/// Obtiene el estado de dark mode actual desde gsettings
async fn get_current_dark_mode() -> Result<bool, String> {
    let output = Command::new("gsettings")
        .args(&["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        .map_err(|e| format!("Error obteniendo color scheme: {}", e))?;
    
    let scheme = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();
    
    Ok(scheme.contains("dark"))
}

/// Establece la configuración del sistema y persiste los cambios
#[tauri::command]
pub async fn set_system_config(config: SystemConfig) -> Result<SystemConfig, String> {
    // Aplicar cambios al sistema
    apply_system_config(&config).await?;

    // Persistir configuración en archivo
    let config_path = get_config_path()?;
    
    // Crear directorio si no existe
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Error creando directorio de configuración: {}", e))?;
    }
    
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Error serializando configuración: {}", e))?;
    
    std::fs::write(&config_path, config_json)
        .map_err(|e| format!("Error guardando configuración: {}", e))?;

    Ok(config)
}

/// Obtiene la ruta del archivo de configuración
fn get_config_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME")
        .map_err(|e| format!("Error obteniendo HOME: {}", e))?;
    
    Ok(PathBuf::from(home)
        .join(".config/vasak/system_config.json"))
}

/// Aplica la configuración al sistema
async fn apply_system_config(config: &SystemConfig) -> Result<(), String> {
    // Aplicar tema GTK
    set_gtk_theme(&config.gtk_theme, config.dark_mode).await?;

    // Aplicar cursor
    set_cursor_theme(&config.cursor_theme).await?;

    // Aplicar icon pack
    set_icon_pack(&config.icon_pack).await?;

    // Aplicar modo oscuro/claro
    set_dark_mode(config.dark_mode).await?;

    // Nota: Color primario y border radius se aplican via CSS vars en Vue

    Ok(())
}

/// Cambia el tema GTK del sistema
pub async fn set_gtk_theme(theme: &str, dark_mode: bool) -> Result<(), String> {
    let theme_name = if dark_mode {
        format!("{}-dark", theme)
    } else {
        theme.to_string()
    };

    let output = Command::new("gsettings")
        .args(&["set", "org.gnome.desktop.interface", "gtk-theme", &theme_name])
        .output()
        .map_err(|e| format!("Error setting GTK theme: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Error al aplicar tema GTK: {}", stderr));
    }

    println!("GTK theme aplicado: {}", theme_name);
    Ok(())
}

/// Cambia el cursor del sistema
pub async fn set_cursor_theme(cursor: &str) -> Result<(), String> {
    let output = Command::new("gsettings")
        .args(&["set", "org.gnome.desktop.interface", "cursor-theme", cursor])
        .output()
        .map_err(|e| format!("Error setting cursor theme: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Error al aplicar cursor: {}", stderr));
    }

    println!("Cursor theme aplicado: {}", cursor);
    Ok(())
}

/// Cambia el pack de iconos del sistema
pub async fn set_icon_pack(icon_pack: &str) -> Result<(), String> {
    let output = Command::new("gsettings")
        .args(&["set", "org.gnome.desktop.interface", "icon-theme", icon_pack])
        .output()
        .map_err(|e| format!("Error setting icon pack: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Error al aplicar pack de iconos: {}", stderr));
    }

    println!("Icon pack aplicado: {}", icon_pack);
    Ok(())
}

/// Activa/desactiva el modo oscuro
pub async fn set_dark_mode(dark_mode: bool) -> Result<(), String> {
    let scheme = if dark_mode {
        "prefer-dark"
    } else {
        "prefer-light"
    };

    Command::new("gsettings")
        .args(&["set", "org.gnome.desktop.interface", "color-scheme", scheme])
        .output()
        .map_err(|e| format!("Error setting color scheme: {}", e))?;

    Ok(())
}

/// Obtiene lista de temas GTK disponibles
#[tauri::command]
pub async fn get_gtk_themes() -> Result<Vec<String>, String> {
    let themes_path = PathBuf::from("/usr/share/themes");
    
    if !themes_path.exists() {
        return Ok(vec!["Adwaita".to_string()]);
    }

    let entries = std::fs::read_dir(&themes_path)
        .map_err(|e| format!("Error reading themes: {}", e))?;

    let mut themes = Vec::new();
    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        themes.push(file_name);
                    }
                }
            }
        }
    }

    themes.sort();
    Ok(themes)
}

/// Obtiene lista de temas de cursor disponibles
#[tauri::command]
pub async fn get_cursor_themes() -> Result<Vec<String>, String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let local_icons = PathBuf::from(&home).join(".local/share/icons");
    
    let cursor_paths = vec![
        PathBuf::from("/usr/share/icons"),
        local_icons,
    ];

    let mut cursors = std::collections::HashSet::new();
    cursors.insert("Adwaita".to_string());

    for path in cursor_paths {
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    // Verificar que tenga un subdirectorio "cursors"
                    let cursors_dir = entry.path().join("cursors");
                    if cursors_dir.exists() && cursors_dir.is_dir() {
                        if let Ok(file_name) = entry.file_name().into_string() {
                            cursors.insert(file_name);
                        }
                    }
                }
            }
        }
    }

    let mut result: Vec<String> = cursors.into_iter().collect();
    result.sort();
    Ok(result)
}

/// Obtiene lista de packs de iconos disponibles
#[tauri::command]
pub async fn get_icon_packs() -> Result<Vec<String>, String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let local_icons = PathBuf::from(&home).join(".local/share/icons");
    
    let icon_paths = vec![
        PathBuf::from("/usr/share/icons"),
        local_icons,
    ];

    let mut icons = std::collections::HashSet::new();
    icons.insert("Adwaita".to_string());

    for path in icon_paths {
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let entry_path = entry.path();
                    let index_theme = entry_path.join("index.theme");
                    let cursors_dir = entry_path.join("cursors");
                    
                    // Solo incluir si tiene index.theme Y NO es un cursor theme
                    if index_theme.exists() && !cursors_dir.exists() {
                        if let Ok(file_name) = entry.file_name().into_string() {
                            icons.insert(file_name);
                        }
                    }
                }
            }
        }
    }

    let mut result: Vec<String> = icons.into_iter().collect();
    result.sort();
    Ok(result)
}

