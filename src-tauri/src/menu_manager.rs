use freedesktop_entry_parser::parse_entry;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppEntry {
    category: String,
    name: String,
    generic: String,
    description: String,
    icon: String,
    keywords: String,
    path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryInfo {
    icon: String,
    description: String,
    apps: Vec<AppEntry>,
}

fn normalize_category(categories: &str) -> String {
    let categories: Vec<&str> = categories.split(';').collect();
    
    // Primero intentamos encontrar una categoría específica
    for category in categories.iter() {
        match *category {
            "Development" | "IDE" | "GUIDesigner" | "Programming" | "WebDevelopment" | "Building" | "Debugger" => return "develop".to_string(),
            "Network" | "Internet" | "Email" | "WebBrowser" | "InstantMessaging" | "Chat" | "FileTransfer" | "HamRadio" | "News" | "P2P" | "RemoteAccess" | "Telephony" | "VideoConference" | "Web" => return "network".to_string(),
            "Settings" | "System" | "Administration" | "DesktopSettings" | "HardwareSettings" | "Preferences" | "Security" => return "settings".to_string(),
            "AudioVideo" | "Audio" | "Video" | "Graphics" | "Music" | "Player" | "Recorder" | "DiscBurning" | "Photography" => return "media".to_string(),
            "Game" | "Games" | "Amusement" | "ActionGame" | "AdventureGame" | "ArcadeGame" | "BoardGame" | "BlocksGame" | "CardGame" | "KidsGame" | "LogicGame" | "RolePlaying" | "Shooter" | "Simulation" | "SportsGame" | "StrategyGame" => return "games".to_string(),
            "Utility" | "Accessories" | "TextEditor" | "Calculator" | "Core" | "FileManager" | "Terminal" | "TrayIcon" | "Archive" | "Compression" | "FileTools" | "Viewer" => return "utility".to_string(),
            _ => continue,
        }
    }
    
    // Si no encontramos una categoría específica, retornamos "utility"
    "utility".to_string()
}

pub fn get_menu() -> HashMap<String, CategoryInfo> {
    let mut menu_items: HashMap<String, CategoryInfo> = HashMap::new();
    let apps_dir = Path::new("/usr/share/applications");
    
    // Inicializar todas las categorías con estructuras vacías
    let categories = ["all", "develop", "network", "settings", "media", "games", "utility"];
    for &category in categories.iter() {
        menu_items.insert(category.to_string(), CategoryInfo {
            icon: get_category_icon(category),
            description: get_category_description(category),
            apps: Vec::new(),
        });
    }

    if let Ok(entries) = fs::read_dir(apps_dir) {
        for entry in entries.flatten() {
            if let Ok(path) = entry.path().into_os_string().into_string() {
                if path.ends_with(".desktop") {
                    
                    if let Ok(entry_data) = parse_entry(&path) {
                        let desktop_entry = entry_data.section("Desktop Entry");
                        
                        // Ignorar entradas que no deberían mostrarse
                        if desktop_entry.attr("NoDisplay").unwrap_or("false") == "true" {
                            continue;
                        }

                        let categories = desktop_entry.attr("Categories").unwrap_or("");
                        
                        let normalized_category = normalize_category(categories);
                        
                        let name = desktop_entry.attr("Name").unwrap_or("").to_string();
                        
                        let app_entry = AppEntry {
                            category: normalized_category.clone(),
                            name: name.clone(),
                            generic: desktop_entry.attr("GenericName").unwrap_or("").to_string(),
                            description: desktop_entry.attr("Comment").unwrap_or("").to_string(),
                            icon: desktop_entry.attr("Icon").unwrap_or("").to_string(),
                            keywords: desktop_entry.attr("Keywords").unwrap_or("").to_string(),
                            path: path.clone(),
                        };

                        // Agregar a la categoría específica
                        if let Some(category_info) = menu_items.get_mut(&normalized_category) {
                            category_info.apps.push(app_entry.clone());
                        }

                        // Agregar a "all"
                        if let Some(all_category) = menu_items.get_mut("all") {
                            all_category.apps.push(app_entry);
                        }
                    }
                }
            }
        }
    }

    menu_items
}

fn get_category_icon(category: &str) -> String {
    match category {
        "all" => "applications-all".to_string(),
        "develop" => "applications-development".to_string(),
        "network" => "applications-internet".to_string(),
        "settings" => "preferences-system".to_string(),
        "media" => "applications-multimedia".to_string(),
        "games" => "applications-games".to_string(),
        "utility" => "applications-utilities".to_string(),
        _ => "applications-other".to_string(),
    }
}

fn get_category_description(category: &str) -> String {
    match category {
        "all" => "Todas las aplicaciones".to_string(),
        "develop" => "Herramientas de desarrollo".to_string(),
        "network" => "Internet y redes".to_string(),
        "settings" => "Configuración del sistema".to_string(),
        "media" => "Aplicaciones multimedia".to_string(),
        "games" => "Juegos".to_string(),
        "utility" => "Utilidades".to_string(),
        _ => "Otras aplicaciones".to_string(),
    }
} 