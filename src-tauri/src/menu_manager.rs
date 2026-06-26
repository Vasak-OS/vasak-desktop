use freedesktop_entry_parser::parse_entry;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use crate::logger::log_info;
use crate::structs::{AppEntry, CategoryInfo};

fn get_applications_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for dir in data_dirs.split(':') {
            let apps_dir = PathBuf::from(dir).join("applications");
            if apps_dir.exists() {
                dirs.push(apps_dir);
            }
        }
    } else {
        for dir in &["/usr/share", "/usr/local/share"] {
            let apps_dir = PathBuf::from(dir).join("applications");
            if apps_dir.exists() {
                dirs.push(apps_dir);
            }
        }
    }

    if let Some(home) = dirs::home_dir() {
        let user_apps = home.join(".local/share/applications");
        if user_apps.exists() {
            dirs.push(user_apps);
        }
    }

    dirs
}

fn normalize_category(categories: &str) -> String {
    let categories: Vec<&str> = categories.split(';').collect();

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

    "utility".to_string()
}

pub fn get_menu() -> HashMap<String, CategoryInfo> {
    log_info("Cargando menú de aplicaciones");
    let mut menu_items: HashMap<String, CategoryInfo> = HashMap::new();
    let mut seen_names: HashSet<String> = HashSet::new();

    let categories = ["all", "develop", "network", "settings", "media", "games", "utility"];
    for &category in categories.iter() {
        menu_items.insert(category.to_string(), CategoryInfo {
            icon: get_category_icon(category),
            description: get_category_description(category),
            apps: Vec::new(),
        });
    }

    for apps_dir in get_applications_dirs() {
        if let Ok(entries) = fs::read_dir(&apps_dir) {
            for entry in entries.flatten() {
                let path_str = match entry.path().into_os_string().into_string() {
                    Ok(p) => p,
                    Err(_) => continue,
                };

                if !path_str.ends_with(".desktop") {
                    continue;
                }

                let file_name = match entry.file_name().into_string() {
                    Ok(n) => n,
                    Err(_) => continue,
                };

                if !seen_names.insert(file_name) {
                    continue;
                }

                if let Ok(entry_data) = parse_entry(&path_str) {
                    let desktop_entry = entry_data.section("Desktop Entry");

                    if desktop_entry.attr("NoDisplay").unwrap_or("false") == "true" {
                        continue;
                    }

                    let app_categories = desktop_entry.attr("Categories").unwrap_or("");
                    let normalized_category = normalize_category(app_categories);
                    let name = desktop_entry.attr("Name").unwrap_or("").to_string();

                    let app_entry = AppEntry {
                        category: normalized_category.clone(),
                        name: name.clone(),
                        generic: desktop_entry.attr("GenericName").unwrap_or("").to_string(),
                        description: desktop_entry.attr("Comment").unwrap_or("").to_string(),
                        icon: desktop_entry.attr("Icon").unwrap_or("").to_string(),
                        keywords: desktop_entry.attr("Keywords").unwrap_or("").to_string(),
                        path: path_str.clone(),
                    };

                    if let Some(category_info) = menu_items.get_mut(&normalized_category) {
                        category_info.apps.push(app_entry.clone());
                    }

                    if let Some(all_category) = menu_items.get_mut("all") {
                        all_category.apps.push(app_entry);
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
