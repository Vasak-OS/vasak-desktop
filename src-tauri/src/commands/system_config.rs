use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub dark_mode: bool,
    pub icon_pack: String,
    pub cursor_theme: String,
    pub gtk_theme: String,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
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

        serde_json::from_str(&content).map_err(|e| format!("Error parseando configuración: {}", e))
    } else {
        eprintln!("[get_system_config] Archivo no existe, usando defaults");
        Ok(SystemConfig::default())
    }
}

#[tauri::command]
pub async fn get_current_system_state() -> Result<SystemConfig, String> {
    let gtk_theme = get_current_gtk_theme()
        .await
        .unwrap_or_else(|_| "Adwaita".to_string());
    let cursor_theme = get_current_cursor_theme()
        .await
        .unwrap_or_else(|_| "Adwaita".to_string());
    let icon_pack = get_current_icon_pack()
        .await
        .unwrap_or_else(|_| "Adwaita".to_string());
    let dark_mode = get_current_dark_mode().await.unwrap_or(false);

    Ok(SystemConfig {
        dark_mode,
        icon_pack,
        cursor_theme,
        gtk_theme,
    })
}

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

    let scheme = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok(scheme.contains("dark"))
}

/// Establece la configuración del sistema y persiste los cambios
#[tauri::command]
pub async fn set_system_config(config: SystemConfig) -> Result<SystemConfig, String> {
    apply_system_config(&config).await?;

    let config_path = get_config_path()?;

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
    let home = std::env::var("HOME").map_err(|e| format!("Error obteniendo HOME: {}", e))?;

    Ok(PathBuf::from(home).join(".config/vasak/system_config.json"))
}

async fn apply_system_config(config: &SystemConfig) -> Result<(), String> {
    if let Err(e) = set_gtk_theme(&config.gtk_theme, config.dark_mode).await {
        eprintln!("⚠️  Error GTK (no crítico): {}", e);
    }

    if let Err(e) = set_cursor_theme(&config.cursor_theme).await {
        eprintln!("⚠️  Error Cursor (no crítico): {}", e);
    }

    set_icon_pack(&config.icon_pack).await?;
    set_dark_mode(config.dark_mode).await?;

    Ok(())
}

pub async fn set_gtk_theme(theme: &str, _dark_mode: bool) -> Result<(), String> {
    let output = Command::new("gsettings")
        .args(&[
            "set",
            "org.gnome.desktop.interface",
            "gtk-theme",
            &theme.to_string(),
        ])
        .output()
        .map_err(|e| format!("Error setting GTK theme: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Error al aplicar tema GTK: {}", stderr));
    }

    println!("GTK theme aplicado: {}", &theme.to_string());
    Ok(())
}

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

pub async fn set_icon_pack(icon_pack: &str) -> Result<(), String> {
    let available_packs = get_icon_packs().await?;
    if !available_packs.contains(&icon_pack.to_string()) {
        let msg = format!(
            "Icon pack '{}' no encontrado. Disponibles: {:?}",
            icon_pack, available_packs
        );
        eprintln!("{}", msg);
        return Err(msg);
    }

    let output = Command::new("gsettings")
        .args(&[
            "set",
            "org.gnome.desktop.interface",
            "icon-theme",
            icon_pack,
        ])
        .output()
        .map_err(|e| {
            let err_msg = format!("Error ejecutando gsettings: {}", e);
            eprintln!("{}", err_msg);
            err_msg
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let err_msg = format!(
            "[set_icon_pack] Error: exit_code={}, stderr={}, stdout={}",
            output.status.code().unwrap_or(-1),
            stderr,
            stdout
        );
        eprintln!("{}", err_msg);
        return Err(format!("Error al aplicar pack de iconos: {}", stderr));
    }
    Ok(())
}

pub async fn set_dark_mode(dark_mode: bool) -> Result<(), String> {
    let scheme = if dark_mode {
        "prefer-dark"
    } else {
        "prefer-light"
    };

    let output = Command::new("gsettings")
        .args(&["set", "org.gnome.desktop.interface", "color-scheme", scheme])
        .output()
        .map_err(|e| format!("Error setting color scheme: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("[set_dark_mode] Error: {}", stderr);
        return Err(format!("Error al aplicar esquema de color: {}", stderr));
    }
    Ok(())
}

#[tauri::command]
pub async fn get_gtk_themes() -> Result<Vec<String>, String> {
    let themes_path = PathBuf::from("/usr/share/themes");

    if !themes_path.exists() {
        return Ok(vec!["Adwaita".to_string()]);
    }

    let entries =
        std::fs::read_dir(&themes_path).map_err(|e| format!("Error reading themes: {}", e))?;

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

#[tauri::command]
pub async fn get_cursor_themes() -> Result<Vec<String>, String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let local_icons = PathBuf::from(&home).join(".local/share/icons");

    let cursor_paths = vec![PathBuf::from("/usr/share/icons"), local_icons];

    let mut cursors = std::collections::HashSet::new();
    cursors.insert("Adwaita".to_string());

    for path in cursor_paths {
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries {
                if let Ok(entry) = entry {
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

#[tauri::command]
pub async fn get_icon_packs() -> Result<Vec<String>, String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let local_icons = PathBuf::from(&home).join(".local/share/icons");

    let icon_paths = vec![PathBuf::from("/usr/share/icons"), local_icons];

    let mut icons = std::collections::HashSet::new();
    icons.insert("Adwaita".to_string());

    for path in icon_paths {
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let entry_path = entry.path();
                    let index_theme = entry_path.join("index.theme");
                    let cursors_dir = entry_path.join("cursors");

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
