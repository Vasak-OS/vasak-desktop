use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use crate::logger::{log_info, log_error};

/// El directorio desde el que arranca una aplicación que abre el escritorio.
///
/// Tiene que ser el hogar y no el que herede el proceso del escritorio. Un
/// proceso hijo hereda el directorio de trabajo del padre, y el del escritorio
/// es el que haya tenido quien lo lanzó: con la sesión normal es el hogar, pero
/// arrancado a mano desde el árbol de fuentes de una aplicación Tauri —cosa que
/// se hace todo el tiempo para probar— es un directorio con un `locales/`
/// adentro.
///
/// Y eso rompe a la aplicación que se abra: el plugin de idiomas busca los
/// catálogos primero en `<directorio actual>/locales`, así que **cualquier**
/// aplicación abierta desde el menú cargaba los textos del escritorio en lugar
/// de los suyos y mostraba las claves crudas —`app.titulo`, `recursos.cpu`— en
/// vez de la interfaz traducida. El mismo binario, abierto desde otro lado,
/// andaba perfecto.
fn directorio_de_arranque() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

fn parse_exec_line(exec_line: &str) -> Result<(String, Vec<String>), String> {
    let parts = shlex::split(exec_line).ok_or_else(|| "No se pudo parsear Exec".to_string())?;

    if parts.is_empty() {
        return Err("Exec vacío".to_string());
    }

    let command = parts[0].clone();
    let args = parts
        .into_iter()
        .skip(1)
        .filter(|arg| !(arg.starts_with('%') && arg.len() == 2))
        .map(|arg| arg.replace("%%", "%"))
        .collect();

    Ok((command, args))
}

#[allow(clippy::lines_filter_map_ok)]
#[tauri::command]
pub async fn open_app(path: &str) -> Result<(), String> {
    log_info(&format!("Abriendo aplicación desde: {}", path));
    let file = File::open(path).map_err(|e| {
        log_error(&format!("Error al abrir archivo .desktop {}: {}", path, e));
        e.to_string()
    })?;
    let reader = BufReader::new(file);

    for line in reader.lines().flatten() {
        if line.starts_with("Exec=") {
            let exec_line = line.trim_start_matches("Exec=");
            let (cmd, args) = parse_exec_line(exec_line).map_err(|e| {
                log_error(&format!("Error parseando Exec en {}: {}", path, e));
                e
            })?;

            log_info(&format!("Ejecutando comando: {} {:?}", cmd, args));
            Command::new(&cmd)
                .args(&args)
                .current_dir(directorio_de_arranque())
                .spawn()
                .map_err(|e| {
                    log_error(&format!("Error al ejecutar comando {} {:?}: {}", cmd, args, e));
                    e.to_string()
                })?;

            return Ok(());
        }
    }

    log_error(&format!("No se encontró comando ejecutable en: {}", path));
    Err("No se encontró el comando ejecutable".to_string())
}

/// Binary shipped by the vasak-settings package.
const SETTINGS_BINARY: &str = "vasak-settings";

/// Launches the VasakOS settings application.
///
/// The menu's gear button invoked `open_configuration_window`, a command that
/// was never implemented on the Rust side: the call rejected, the error was
/// swallowed by the caller's catch, and the button silently did nothing. The
/// shell has no settings UI of its own — vasak-settings is a separate app — so
/// the button's job is simply to launch it.
pub fn spawn_settings() -> Result<(), String> {
    spawn_settings_at(None)
}

/// La misma aplicación, abierta en una sección puntual.
///
/// `vasak-settings appearance-panel` abre esa pantalla en vez de la portada: el
/// menú del panel lleva al ajuste que corresponde en vez de dejar a la persona
/// buscándolo en el menú lateral. La sección viaja como argumento y la valida el
/// otro lado; acá se filtra por forma para no pasarle cualquier cosa.
pub fn spawn_settings_at(seccion: Option<&str>) -> Result<(), String> {
    log_info("Abriendo la aplicación de configuración");

    let mut comando = Command::new(SETTINGS_BINARY);
    comando.current_dir(directorio_de_arranque());

    if let Some(seccion) = seccion.filter(|valor| es_nombre_de_seccion(valor)) {
        comando.arg(seccion);
    }

    comando.spawn().map_err(|error| {
        let message = format!(
            "No se pudo abrir {}: {}. ¿Está instalado el paquete vasak-settings?",
            SETTINGS_BINARY, error
        );
        log_error(&message);
        message
    })?;

    Ok(())
}

#[tauri::command]
pub async fn open_settings() -> Result<(), String> {
    spawn_settings()
}

/// La configuración, abierta en una sección: lo usa el menú del panel.
#[tauri::command]
pub async fn open_settings_section(section: String) -> Result<(), String> {
    spawn_settings_at(Some(&section))
}

/// Minúsculas, dígitos y guiones: los nombres que usa el router de la
/// configuración. No es una barrera de seguridad —el argumento va como un
/// elemento propio de `argv`, no por una shell— sino la forma de no arrastrar
/// hasta otra aplicación algo que claramente no es una sección.
fn es_nombre_de_seccion(valor: &str) -> bool {
    // Que no empiece con guion no es cosmético: `vasak-settings --help` trataría
    // el argumento como una opción de la aplicación en vez de como la pantalla
    // que hay que abrir.
    !valor.is_empty()
        && !valor.starts_with('-')
        && valor.len() <= 40
        && valor
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests_arranque {
    use super::directorio_de_arranque;

    #[test]
    fn es_un_directorio_que_existe_y_no_el_del_escritorio() {
        // Lo que importa no es cuál sea, sino que sea uno elegido y no el que el
        // escritorio haya heredado: desde el árbol de fuentes de una aplicación
        // Tauri, el directorio actual tiene un `locales/` que la aplicación
        // abierta cargaba en lugar de los suyos.
        let directorio = directorio_de_arranque();
        assert!(directorio.is_absolute(), "{directorio:?}");
        assert!(directorio.is_dir(), "{directorio:?}");

        if let Ok(actual) = std::env::current_dir() {
            if actual.join("locales").is_dir() {
                assert_ne!(
                    directorio, actual,
                    "se estaría entregando un directorio con catálogos de otra aplicación"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests_seccion {
    use super::es_nombre_de_seccion;

    #[test]
    fn las_secciones_reales_pasan() {
        assert!(es_nombre_de_seccion("appearance-panel"));
        assert!(es_nombre_de_seccion("network-wifi"));
    }

    #[test]
    fn lo_que_no_es_una_seccion_no_pasa() {
        assert!(!es_nombre_de_seccion(""));
        assert!(!es_nombre_de_seccion("/etc/passwd"));
        assert!(!es_nombre_de_seccion("Appearance-Panel"));
        assert!(!es_nombre_de_seccion("dos palabras"));
        assert!(!es_nombre_de_seccion("--help"), "una opción, no una sección");
        assert!(!es_nombre_de_seccion(&"a".repeat(41)));
    }
}
