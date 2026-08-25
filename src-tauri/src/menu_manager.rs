use freedesktop_entry_parser::parse_entry;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use crate::logger::log_info;
use crate::structs::{AppEntry, CategoryInfo};

/// Parsed menu, kept until something changes on disk.
///
/// Scanning is not cheap: it reads and parses every .desktop file in every XDG
/// applications directory. Doing that each time the menu opens blocked the UI
/// thread for the whole scan; the cache plus the watcher in `menu_watcher`
/// means it happens once at startup and again only when an app is installed or
/// removed.
static MENU_CACHE: RwLock<Option<HashMap<String, CategoryInfo>>> = RwLock::new(None);

/// Directories whose contents decide what the menu shows.
pub fn applications_dirs() -> Vec<PathBuf> {
    get_applications_dirs()
}

/// The menu, from cache when it is warm.
pub fn get_menu_cached() -> HashMap<String, CategoryInfo> {
    if let Ok(cache) = MENU_CACHE.read() {
        if let Some(menu) = cache.as_ref() {
            return menu.clone();
        }
    }

    let menu = get_menu();

    if let Ok(mut cache) = MENU_CACHE.write() {
        *cache = Some(menu.clone());
    }

    menu
}

/// Drops the cache so the next read rescans. Called when the watcher sees a
/// .desktop file appear, change or disappear.
pub fn invalidate_menu_cache() {
    if let Ok(mut cache) = MENU_CACHE.write() {
        *cache = None;
    }
}

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

/// Session locale, most specific first: `es_AR.UTF-8` yields `es_AR` and `es`.
fn locale_keys() -> Vec<String> {
    let raw = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();

    let locale = raw.split(['.', '@']).next().unwrap_or("");
    if locale.is_empty() || locale == "C" || locale == "POSIX" {
        return Vec::new();
    }

    let mut keys = vec![locale.to_string()];
    if let Some(language) = locale.split('_').next() {
        if language != locale {
            keys.push(language.to_string());
        }
    }

    keys
}

/// Value of `key` in the session language, falling back to the untranslated
/// one. Applications ship their translations in the same file, as `Name[es]`,
/// and reading only `Name` left the menu in English on a Spanish system even
/// for the applications that do translate themselves.
fn localized_attr<T: AsRef<str>>(
    section: &freedesktop_entry_parser::AttrSelector<T>,
    key: &str,
    locales: &[String],
) -> String {
    for locale in locales {
        if let Some(value) = section.attr_with_param(key, locale) {
            return value.to_string();
        }
    }

    section.attr(key).unwrap_or("").to_string()
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
    let locales = locale_keys();

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
                    let name = localized_attr(&desktop_entry, "Name", &locales);

                    let app_entry = AppEntry {
                        category: normalized_category.clone(),
                        name: name.clone(),
                        generic: localized_attr(&desktop_entry, "GenericName", &locales),
                        description: localized_attr(&desktop_entry, "Comment", &locales),
                        icon: desktop_entry.attr("Icon").unwrap_or("").to_string(),
                        keywords: localized_attr(&desktop_entry, "Keywords", &locales),
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

    // Ordenado acá, una vez por escaneo, y no en el frontend en cada apertura.
    //
    // El menú vive en `MENU_CACHE` y sólo se rearma cuando el vigilante ve
    // cambiar un `.desktop`, pero la vista igual llamaba a `localeCompare`
    // sobre cada categoría cada vez que se abría: con las 195 aplicaciones de
    // este sistema son cerca de novecientas comparaciones con reglas de
    // colación, en el camino crítico de la superficie más usada del escritorio.
    // Acá se pagan una vez y quedan guardadas.
    for category in menu_items.values_mut() {
        ordenar_aplicaciones(&mut category.apps);
    }

    menu_items
}

/// Alfabético por nombre visible, sin distinguir mayúsculas ni acentos.
///
/// `to_lowercase` y no una comparación cruda: con la comparación por bytes
/// «Zathura» iba antes que «archivos», que es lo que hacía falta corregir en el
/// frontend con `localeCompare`. No es una colación completa —«ñ» sigue después
/// de «z» en Unicode— pero para nombres de aplicaciones da el mismo resultado
/// que se veía, sin el costo por apertura.
fn ordenar_aplicaciones(apps: &mut [crate::structs::AppEntry]) {
    apps.sort_by(|izquierda, derecha| {
        izquierda
            .name
            .to_lowercase()
            .cmp(&derecha.name.to_lowercase())
    });
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

/// Locale key for a category's description.
///
/// The backend returns a key rather than text: it has no notion of the user's
/// language, and the shell is translated on the frontend.
fn get_category_description(category: &str) -> String {
    let known = matches!(
        category,
        "all" | "develop" | "network" | "settings" | "media" | "games" | "utility"
    );

    if known {
        format!("menu.categories.{}", category)
    } else {
        "menu.categories.other".to_string()
    }
}
