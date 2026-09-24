use crate::logger::log_info;
use crate::structs::{AppEntry, CategoryInfo};
use freedesktop_entry_parser::{parse_entry, Section};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

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

/// Lo que usa XDG cuando `XDG_DATA_DIRS` no dice nada.
const DEFAULT_DATA_DIRS: &str = "/usr/local/share:/usr/share";

/// Los directorios de entradas `.desktop`, **del que más manda al que menos**.
///
/// El orden no es cosmético: `get_menu` se queda con la primera entrada de cada
/// nombre de archivo, así que quien vaya primero le gana a los demás. Y estaba
/// al revés —el directorio de quien usa el sistema iba último—, de modo que un
/// `~/.local/share/applications/loquesea.desktop` puesto para cambiarle el
/// nombre, el icono o el comando a una aplicación quedaba tapado por el del
/// sistema y no hacía nada. La especificación XDG dice lo contrario:
/// `XDG_DATA_HOME` manda sobre `XDG_DATA_DIRS`.
///
/// De paso se respeta `XDG_DATA_HOME` en vez de dar por sentado
/// `~/.local/share`, y el orden de la lista por omisión, que también estaba
/// invertido: `/usr/local/share` va antes que `/usr/share`.
fn sort_directories(
    data_home: Option<String>,
    home: Option<PathBuf>,
    data_dirs: Option<String>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // Absoluta o nada. Filtraba la cadena **vacía** y no la **relativa**, que
    // tiene la misma consecuencia —una ruta respecto del directorio de trabajo—
    // y que el estándar manda ignorar igual. Una regla en vez de dos: la cadena
    // vacía tampoco es absoluta, así que los dos casos salen de la misma
    // comprobación.
    let user_base = data_home
        .map(PathBuf::from)
        .filter(|base| base.is_absolute())
        .or_else(|| {
            home.filter(|home_dir| home_dir.is_absolute())
                .map(|home_dir| home_dir.join(".local/share"))
        });

    if let Some(base) = user_base {
        dirs.push(base.join("applications"));
    }

    // Una variable definida pero vacía significa «usá lo de siempre», igual que
    // si no estuviera.
    let system_dirs = data_dirs.filter(|value| !value.is_empty());

    for dir in system_dirs
        .as_deref()
        .unwrap_or(DEFAULT_DATA_DIRS)
        .split(':')
        // Lo mismo para cada entrada de la lista: una relativa se ignora, y la
        // vacía es un caso de esa misma regla.
        .filter(|dir| Path::new(dir).is_absolute())
    {
        let apps_dir = PathBuf::from(dir).join("applications");
        if !dirs.contains(&apps_dir) {
            dirs.push(apps_dir);
        }
    }

    dirs
}

fn get_applications_dirs() -> Vec<PathBuf> {
    sort_directories(
        std::env::var("XDG_DATA_HOME").ok(),
        dirs::home_dir(),
        std::env::var("XDG_DATA_DIRS").ok(),
    )
    .into_iter()
    .filter(|dir| dir.exists())
    .collect()
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
fn localized_attr(section: &Section, key: &str, locales: &[String]) -> String {
    for locale in locales {
        if let Some(value) = section.attr_with_param(key, locale).first() {
            return value.clone();
        }
    }

    first_attr(section, key).unwrap_or("").to_string()
}

/// El valor de `key`, si el archivo lo tiene.
///
/// Desde la versión 2 el parser devuelve todas las apariciones de una clave —una
/// lista, no un valor— para que una clave repetida no se pierda. Para el menú
/// vale la primera, que es lo que devolvía la versión 1.
pub(crate) fn first_attr<'a>(section: &'a Section, key: &str) -> Option<&'a str> {
    section.attr(key).first().map(String::as_str)
}

fn normalize_category(categories: &str) -> String {
    let categories: Vec<&str> = categories.split(';').collect();

    for category in categories.iter() {
        match *category {
            "Development" | "IDE" | "GUIDesigner" | "Programming" | "WebDevelopment"
            | "Building" | "Debugger" => return "develop".to_string(),
            "Network" | "Internet" | "Email" | "WebBrowser" | "InstantMessaging" | "Chat"
            | "FileTransfer" | "HamRadio" | "News" | "P2P" | "RemoteAccess" | "Telephony"
            | "VideoConference" | "Web" => return "network".to_string(),
            "Settings" | "System" | "Administration" | "DesktopSettings" | "HardwareSettings"
            | "Preferences" | "Security" => return "settings".to_string(),
            "AudioVideo" | "Audio" | "Video" | "Graphics" | "Music" | "Player" | "Recorder"
            | "DiscBurning" | "Photography" => return "media".to_string(),
            "Game" | "Games" | "Amusement" | "ActionGame" | "AdventureGame" | "ArcadeGame"
            | "BoardGame" | "BlocksGame" | "CardGame" | "KidsGame" | "LogicGame"
            | "RolePlaying" | "Shooter" | "Simulation" | "SportsGame" | "StrategyGame" => {
                return "games".to_string()
            }
            "Utility" | "Accessories" | "TextEditor" | "Calculator" | "Core" | "FileManager"
            | "Terminal" | "TrayIcon" | "Archive" | "Compression" | "FileTools" | "Viewer" => {
                return "utility".to_string()
            }
            _ => continue,
        }
    }

    "utility".to_string()
}

pub fn get_menu() -> HashMap<String, CategoryInfo> {
    log_info("Cargando menú de aplicaciones");
    menu_from_dirs(&get_applications_dirs())
}

/// El menú armado a partir de estos directorios, en orden de prioridad: de cada
/// nombre de archivo gana el primero que aparece.
fn menu_from_dirs(dirs: &[std::path::PathBuf]) -> HashMap<String, CategoryInfo> {
    let mut menu_items: HashMap<String, CategoryInfo> = HashMap::new();
    let mut seen_names: HashSet<String> = HashSet::new();
    let locales = locale_keys();

    let categories = [
        "all", "develop", "network", "settings", "media", "games", "utility",
    ];
    for &category in categories.iter() {
        menu_items.insert(
            category.to_string(),
            CategoryInfo {
                icon: get_category_icon(category),
                description: get_category_description(category),
                apps: Vec::new(),
            },
        );
    }

    for apps_dir in dirs {
        if let Ok(entries) = fs::read_dir(apps_dir) {
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

                if seen_names.contains(&file_name) {
                    continue;
                }

                if let Ok(entry_data) = parse_entry(&path_str) {
                    // Sin el grupo `Desktop Entry` no hay aplicación que mostrar:
                    // antes el archivo entraba igual, con los campos vacíos.
                    //
                    // Y el nombre se reserva recién después de comprobarlo. Una
                    // copia rota en el directorio del usuario no puede tapar la
                    // del sistema que sí sirve; una con `NoDisplay=true` sí la
                    // tapa, porque esconderla es justamente lo que pidió.
                    let Some(desktop_entry) = entry_data.section("Desktop Entry") else {
                        continue;
                    };

                    seen_names.insert(file_name);

                    if first_attr(desktop_entry, "NoDisplay").unwrap_or("false") == "true" {
                        continue;
                    }

                    let app_categories = first_attr(desktop_entry, "Categories").unwrap_or("");
                    let normalized_category = normalize_category(app_categories);
                    let name = localized_attr(desktop_entry, "Name", &locales);

                    let app_entry = AppEntry {
                        category: normalized_category.clone(),
                        name: name.clone(),
                        generic: localized_attr(desktop_entry, "GenericName", &locales),
                        description: localized_attr(desktop_entry, "Comment", &locales),
                        icon: first_attr(desktop_entry, "Icon").unwrap_or("").to_string(),
                        keywords: localized_attr(desktop_entry, "Keywords", &locales),
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
        sort_applications(&mut category.apps);
    }

    menu_items
}

/// Alfabético por nombre visible, **sin distinguir mayúsculas**.
///
/// Los acentos **sí** cuentan, y conviene saberlo: se comparan por su valor
/// Unicode, así que «álgebra» queda después de «avahi» y la «ñ» después de la
/// «z». Una colación completa necesita una tabla de reglas por idioma —es lo que
/// hacía `localeCompare` en la vista— y costaba 0,75 ms en cada apertura del
/// menú. Con nombres de aplicaciones el caso se da poco; si alguna vez molesta,
/// lo que corresponde es normalizar los diacríticos acá, no volver a ordenar en
/// la vista.
///
/// `to_lowercase` y no una comparación cruda: con la comparación por bytes
/// «Zathura» iba antes que «archivos», que es lo que hacía falta corregir en el
/// frontend con `localeCompare`. No es una colación completa —«ñ» sigue después
/// de «z» en Unicode— pero para nombres de aplicaciones da el mismo resultado
/// que se veía, sin el costo por apertura.
fn sort_applications(apps: &mut [crate::structs::AppEntry]) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn apps(base: &str) -> PathBuf {
        PathBuf::from(base).join("applications")
    }

    /// El caso que estaba al revés: una entrada del usuario tiene que ganarle a
    /// la del sistema, porque `get_menu` se queda con la primera de cada
    /// nombre.
    #[test]
    fn el_directorio_del_usuario_va_primero() {
        let dirs = sort_directories(
            None,
            Some(PathBuf::from("/home/alguien")),
            Some("/usr/local/share:/usr/share".into()),
        );

        assert_eq!(dirs.first(), Some(&apps("/home/alguien/.local/share")));
    }

    #[test]
    fn se_respeta_el_orden_de_xdg_data_dirs() {
        let dirs = sort_directories(
            None,
            Some(PathBuf::from("/home/alguien")),
            Some("/primero:/segundo:/tercero".into()),
        );

        assert_eq!(
            dirs,
            vec![
                apps("/home/alguien/.local/share"),
                apps("/primero"),
                apps("/segundo"),
                apps("/tercero"),
            ]
        );
    }

    #[test]
    fn xdg_data_home_le_gana_a_la_carpeta_de_siempre() {
        let dirs = sort_directories(
            Some("/otro/lado".into()),
            Some(PathBuf::from("/home/alguien")),
            Some("/usr/share".into()),
        );

        assert_eq!(dirs.first(), Some(&apps("/otro/lado")));
        assert!(!dirs.contains(&apps("/home/alguien/.local/share")));
    }

    /// Una variable definida pero vacía es lo mismo que no tenerla: es lo que
    /// dice la especificación, y si no `"".split(':')` metía un directorio
    /// llamado `applications` colgando de la raíz.
    #[test]
    fn una_variable_vacia_es_como_no_tenerla() {
        let with_empty = sort_directories(
            Some(String::new()),
            Some(PathBuf::from("/home/alguien")),
            Some(String::new()),
        );
        let without_them = sort_directories(None, Some(PathBuf::from("/home/alguien")), None);

        assert_eq!(with_empty, without_them);
        assert!(!with_empty.contains(&apps("")));
    }

    /// `/usr/local/share` antes que `/usr/share`, que es el orden de XDG. La
    /// lista de reserva los tenía al revés.
    #[test]
    fn la_lista_de_reserva_sigue_el_orden_de_xdg() {
        let dirs = sort_directories(None, Some(PathBuf::from("/home/alguien")), None);

        assert_eq!(
            dirs,
            vec![
                apps("/home/alguien/.local/share"),
                apps("/usr/local/share"),
                apps("/usr/share"),
            ]
        );
    }

    #[test]
    fn sin_casa_quedan_solo_los_del_sistema() {
        let dirs = sort_directories(None, None, Some("/usr/share".into()));

        assert_eq!(dirs, vec![apps("/usr/share")]);
    }

    /// Un directorio repetido en `XDG_DATA_DIRS` no puede aparecer dos veces:
    /// no cambia qué gana, pero hace que cada entrada de ahí se lea dos veces.
    #[test]
    fn no_se_repiten_directorios() {
        let dirs = sort_directories(
            Some("/casa".into()),
            None,
            Some("/casa:/usr/share:/usr/share".into()),
        );

        assert_eq!(dirs, vec![apps("/casa"), apps("/usr/share")]);
    }

    #[test]
    fn una_base_relativa_del_usuario_no_entra() {
        // Filtraba la cadena vacía y no una ruta relativa, que tiene la misma
        // consecuencia: se resolvería contra el directorio de trabajo del
        // proceso, que en el escritorio no es el home de nadie.
        //
        // Las cuatro formas de no ser absoluta; la del nombre suelto es la que
        // se escapa cuando uno se acuerda sólo de la vacía.
        for relative in ["", "datos", "./datos", "../datos"] {
            let dirs = sort_directories(Some(relative.into()), None, Some("/usr/share".into()));
            assert_eq!(
                dirs,
                vec![PathBuf::from("/usr/share/applications")],
                "«{relative}» no tiene que aportar un directorio"
            );
        }
    }

    #[test]
    fn un_hogar_relativo_tampoco() {
        let dirs = sort_directories(None, Some(PathBuf::from("casa")), Some("/usr/share".into()));
        assert_eq!(dirs, vec![PathBuf::from("/usr/share/applications")]);
    }

    #[test]
    fn una_entrada_relativa_de_la_lista_del_sistema_se_ignora() {
        // Un `.` acá sería «el directorio desde el que se lanzó el escritorio».
        let dirs = sort_directories(None, None, Some(".:..:relativo:/usr/share".into()));
        assert_eq!(dirs, vec![PathBuf::from("/usr/share/applications")]);
    }

    fn entry(text: &str) -> freedesktop_entry_parser::Entry {
        freedesktop_entry_parser::Entry::parse(text).expect("parsea")
    }

    /// La versión 2 del parser devuelve listas; la lista de categorías tiene
    /// que seguir llegando entera, como una sola cadena, y no partida.
    #[test]
    fn las_categorias_llegan_enteras() {
        let parsed = entry("[Desktop Entry]\nName=Terminal\nCategories=System;TerminalEmulator;\n");
        let section = parsed.section("Desktop Entry").expect("tiene el grupo");

        assert_eq!(
            first_attr(section, "Categories"),
            Some("System;TerminalEmulator;")
        );
        assert_eq!(
            first_attr(section, "Icon"),
            None,
            "una clave ausente es None"
        );
    }

    #[test]
    fn el_nombre_traducido_gana_y_si_no_esta_vale_el_original() {
        let parsed = entry("[Desktop Entry]\nName=Files\nName[es]=Archivos\nComment=Browse\n");
        let section = parsed.section("Desktop Entry").expect("tiene el grupo");
        let locales = vec!["es".to_string()];

        assert_eq!(localized_attr(section, "Name", &locales), "Archivos");
        assert_eq!(localized_attr(section, "Comment", &locales), "Browse");
        assert_eq!(localized_attr(section, "GenericName", &locales), "");
    }

    #[test]
    fn sin_el_grupo_desktop_entry_no_hay_seccion() {
        let parsed = entry("[Otra Cosa]\nName=Nada\n");
        assert!(parsed.section("Desktop Entry").is_none());
    }

    /// Dos directorios de aplicaciones de mentira: el del usuario y el del
    /// sistema, en ese orden de prioridad.
    fn user_and_system_dirs(label: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("vasak-menu-{}-{label}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let user = base.join("user");
        let system = base.join("system");
        fs::create_dir_all(&user).expect("crea el directorio del usuario");
        fs::create_dir_all(&system).expect("crea el directorio del sistema");
        fs::write(
            system.join("editor.desktop"),
            "[Desktop Entry]\nName=Editor\nCategories=Utility;\n",
        )
        .expect("escribe la entrada del sistema");
        (user, system)
    }

    fn names(menu: &HashMap<String, CategoryInfo>) -> Vec<String> {
        menu["all"]
            .apps
            .iter()
            .map(|app| app.name.clone())
            .collect()
    }

    #[test]
    fn una_copia_rota_del_usuario_no_tapa_la_del_sistema() {
        let (user, system) = user_and_system_dirs("rota");
        fs::write(user.join("editor.desktop"), "[Otra Cosa]\nName=Rota\n")
            .expect("escribe la copia rota");

        assert_eq!(names(&menu_from_dirs(&[user, system])), vec!["Editor"]);
    }

    #[test]
    fn una_copia_del_usuario_con_nodisplay_si_la_esconde() {
        let (user, system) = user_and_system_dirs("oculta");
        fs::write(
            user.join("editor.desktop"),
            "[Desktop Entry]\nName=Editor\nNoDisplay=true\n",
        )
        .expect("escribe la copia oculta");

        assert!(names(&menu_from_dirs(&[user, system])).is_empty());
    }
}
