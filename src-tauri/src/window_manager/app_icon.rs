//! El nombre del icono de una ventana, sacado de su archivo `.desktop`.
//!
//! El identificador que da el compositor (`app-id`) casi nunca es el nombre del
//! icono. Antes se derivaba a mano quedándose con el último tramo después del
//! punto, y eso rompía justo en los identificadores de tipo DNS invertido que
//! son la norma: `org.telegram.desktop` daba `desktop`, que en el tema de
//! iconos es la carpeta del escritorio, así que Telegram aparecía en el panel
//! con un icono de carpeta —o sea, no aparecía—.
//!
//! Lo que corresponde es lo que hace cualquier barra de tareas: buscar la
//! entrada `.desktop` de la aplicación y leer su clave `Icon`. Para
//! `org.telegram.desktop` eso da `org.telegram.desktop`; para
//! `com.anthropic.Claude`, `claude-desktop`.
//!
//! Buscar es barato: el `app-id` suele ser el nombre del archivo, así que se
//! prueba `<dir>/<app-id>.desktop` directamente, sin recorrer nada. Sólo cuando
//! eso no da con la entrada se recorren los directorios —por la caja del nombre
//! o por `StartupWMClass`—, y el resultado, incluso el negativo, queda
//! memorizado: cada `app-id` se resuelve una vez y no en cada evento del
//! compositor, que con el panel abierto llegan de a decenas.

use freedesktop_entry_parser::parse_entry;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, RwLock};

use crate::menu_manager::applications_dirs;

/// Icono para cuando no hay nada mejor que ofrecer.
pub const FALLBACK_ICON: &str = "application-x-executable";

/// `app-id` ya resuelto → nombre de icono. `None` es «se buscó y no está»,
/// que también conviene recordar para no volver a recorrer los directorios.
static RESOLVED: LazyLock<RwLock<HashMap<String, Option<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Techo de lo memorizado. La clave la elige el cliente —el `app-id` es lo que
/// la aplicación quiera declarar—, así que sin esto crece hasta donde la dejen.
/// Con más aplicaciones abiertas que esto, el escritorio tiene otro problema.
const LIMITE_MEMORIZADO: usize = 512;

/// Cuántas veces se invalidó lo memorizado. La búsqueda se hace **fuera** del
/// cerrojo —leer archivos con el cerrojo de escritura tomado dejaría al panel
/// esperando—, así que una invalidación puede caer justo en el medio: sin este
/// contador, la búsqueda que empezó antes guardaría después su resultado viejo,
/// y ese icono ya inválido se quedaría hasta el próximo cambio en el disco.
static GENERACION: AtomicU64 = AtomicU64::new(0);

/// Olvida lo resuelto. La llama el vigilante de `.desktop`: una aplicación
/// recién instalada tiene que poder aparecer con su icono, y una que cambió el
/// suyo tiene que dejar de mostrar el viejo.
pub fn invalidate_icon_cache() {
    GENERACION.fetch_add(1, Ordering::SeqCst);
    if let Ok(mut cache) = RESOLVED.write() {
        cache.clear();
    }
}

/// El nombre de icono para un `app-id`, o `None` si no hay entrada `.desktop`
/// que lo reclame.
pub fn icon_for_app_id(app_id: &str) -> Option<String> {
    let key = app_id.trim();
    if key.is_empty() {
        return None;
    }

    if let Ok(cache) = RESOLVED.read() {
        if let Some(hit) = cache.get(key) {
            return hit.clone();
        }
    }

    // La generación se lee antes de buscar: si alguien invalida mientras se
    // leen los `.desktop`, lo que se encontró ya no vale y no se guarda.
    let generacion = GENERACION.load(Ordering::SeqCst);
    let resolved = lookup_icon(key);
    memorizar(key, resolved.clone(), generacion);

    resolved
}

/// Guarda lo resuelto, salvo que se haya invalidado mientras se buscaba.
/// Devuelve si lo guardó.
fn memorizar(key: &str, resolved: Option<String>, generacion: u64) -> bool {
    if GENERACION.load(Ordering::SeqCst) != generacion {
        return false;
    }

    match RESOLVED.write() {
        Ok(mut cache) => {
            if cache.len() >= LIMITE_MEMORIZADO {
                cache.clear();
            }
            cache.insert(key.to_string(), resolved);
            true
        }
        Err(_) => false,
    }
}

/// La clave `Icon` de la entrada `.desktop` que corresponde a este `app-id`.
fn lookup_icon(app_id: &str) -> Option<String> {
    lookup_icon_in(app_id, &applications_dirs())
}

/// Lo mismo, sobre una lista de directorios dada. Separado para poder probarlo
/// sin tocar el entorno del proceso.
fn lookup_icon_in(app_id: &str, dirs: &[std::path::PathBuf]) -> Option<String> {
    // Por nombre de archivo, que es el caso normal y no cuesta un escaneo.
    // Se prueba tal cual y en minúsculas: `com.anthropic.Claude` viene con
    // mayúsculas, otros identificadores llegan con la caja cambiada.
    let lower = app_id.to_lowercase();
    let mut stems = vec![app_id.to_string()];
    if lower != app_id {
        stems.push(lower);
    }

    for dir in dirs {
        for stem in &stems {
            let candidate = dir.join(format!("{stem}.desktop"));
            if let Some(icon) = read_icon(&candidate) {
                return Some(icon);
            }
        }
    }

    // Y si no, un recorrido: la caja del nombre del archivo puede no coincidir
    // con la del `app-id`, y está `StartupWMClass`, que es la clave que existe
    // justamente para atar una ventana a su entrada. Las dos cosas se resuelven
    // en el mismo recorrido, que además se hace una sola vez por `app-id`
    // porque el resultado queda memorizado.
    for dir in dirs {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
                continue;
            }

            let same_stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| stem.eq_ignore_ascii_case(app_id));

            let parsed = match parse_entry(&path) {
                Ok(parsed) => parsed,
                Err(_) => continue,
            };
            let section = parsed.section("Desktop Entry");

            let same_class = section
                .attr("StartupWMClass")
                .is_some_and(|class| class.eq_ignore_ascii_case(app_id));

            if same_stem || same_class {
                if let Some(icon) = non_empty(section.attr("Icon")) {
                    return Some(icon);
                }
            }
        }
    }

    None
}

/// La clave `Icon` de un archivo, si el archivo existe y la tiene.
fn read_icon(path: &Path) -> Option<String> {
    let parsed = parse_entry(path).ok()?;
    non_empty(parsed.section("Desktop Entry").attr("Icon"))
}

fn non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|icon| !icon.is_empty())
        .map(str::to_string)
}

/// El `app-id` como nombre de icono, para cuando no hay entrada `.desktop`.
///
/// Sin recortes por el punto: muchos temas guardan el icono con el
/// identificador completo (`org.telegram.desktop.png` está en *hicolor*), y
/// quedarse con el último tramo convertía nombres válidos en otra cosa. Sólo se
/// normaliza lo que no puede ser parte de un nombre de icono.
pub fn fallback_icon_name(raw: &str) -> String {
    let candidate = raw.trim();
    if candidate.is_empty() {
        return String::new();
    }

    candidate.replace(['_', ' '], "-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn escribir(dir: &Path, nombre: &str, cuerpo: &str) {
        fs::write(dir.join(nombre), cuerpo).expect("no se pudo escribir la entrada");
    }

    /// Un directorio de aplicaciones de mentira, con las entradas que hacen
    /// falta para las pruebas.
    fn directorio_de_aplicaciones(etiqueta: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vasak-app-icon-{}-{etiqueta}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("no se pudo crear el directorio");

        escribir(
            &dir,
            "org.telegram.desktop.desktop",
            "[Desktop Entry]\nName=Telegram\nIcon=org.telegram.desktop\nStartupWMClass=TelegramDesktop\n",
        );
        escribir(
            &dir,
            "com.anthropic.Claude.desktop",
            "[Desktop Entry]\nName=Claude\nIcon=claude-desktop\n",
        );
        escribir(
            &dir,
            "editor.desktop",
            "[Desktop Entry]\nName=Editor\nIcon=editor-icono\nStartupWMClass=EditorRaro\n",
        );
        escribir(
            &dir,
            "sin-icono.desktop",
            "[Desktop Entry]\nName=Pelado\nIcon=\n",
        );
        escribir(&dir, "no-es-entrada.txt", "Icon=trampa\n");

        dir
    }

    #[test]
    fn el_nombre_de_reserva_no_recorta_por_el_punto() {
        // La regresión: `org.telegram.desktop` daba `desktop`, que en el tema de
        // iconos es la carpeta del escritorio, así que Telegram aparecía en el
        // panel con un icono de carpeta —o sea, no aparecía—.
        assert_eq!(
            fallback_icon_name("org.telegram.desktop"),
            "org.telegram.desktop"
        );
        assert_eq!(fallback_icon_name("google-chrome"), "google-chrome");
        assert_eq!(fallback_icon_name(" vasak_terminal "), "vasak-terminal");
        assert_eq!(fallback_icon_name("   "), "");
    }

    #[test]
    fn el_icono_sale_de_la_entrada_desktop() {
        let dir = directorio_de_aplicaciones("entrada");
        let dirs = vec![dir.clone()];

        // Por nombre de archivo, con el identificador completo: esto es lo que
        // pone el icono de Telegram donde tiene que estar.
        assert_eq!(
            lookup_icon_in("org.telegram.desktop", &dirs).as_deref(),
            Some("org.telegram.desktop")
        );

        // La caja del `app-id` no tiene por qué coincidir con la del archivo.
        assert_eq!(
            lookup_icon_in("com.anthropic.claude", &dirs).as_deref(),
            Some("claude-desktop")
        );
        assert_eq!(
            lookup_icon_in("com.anthropic.Claude", &dirs).as_deref(),
            Some("claude-desktop")
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn el_icono_tambien_sale_por_startupwmclass() {
        // Es la clave que existe justamente para atar una ventana a su entrada
        // cuando el `app-id` no es el nombre del archivo.
        let dir = directorio_de_aplicaciones("clase");
        let dirs = vec![dir.clone()];

        assert_eq!(
            lookup_icon_in("editorraro", &dirs).as_deref(),
            Some("editor-icono")
        );
        assert_eq!(
            lookup_icon_in("EditorRaro", &dirs).as_deref(),
            Some("editor-icono")
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lo_que_no_tiene_entrada_no_resuelve() {
        let dir = directorio_de_aplicaciones("sin-entrada");
        let dirs = vec![dir.clone()];

        // Una entrada sin icono no cuenta como encontrada: quien llama tiene un
        // nombre de reserva mejor que la cadena vacía.
        assert_eq!(lookup_icon_in("sin-icono", &dirs), None);
        assert_eq!(lookup_icon_in("no.existe.nada", &dirs), None);
        assert_eq!(lookup_icon_in("", &dirs), None);
        // Un archivo que no es una entrada `.desktop` no se lee.
        assert_eq!(lookup_icon_in("no-es-entrada", &dirs), None);
        // Un directorio inexistente no rompe nada.
        assert_eq!(
            lookup_icon_in("org.telegram.desktop", &[PathBuf::from("/no/existe")]),
            None
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn un_app_id_vacio_no_se_busca() {
        assert_eq!(icon_for_app_id("   "), None);
        assert_eq!(icon_for_app_id(""), None);
    }

    /// Lo memorizado, de punta a punta, en una sola prueba.
    ///
    /// Va junto a propósito: `RESOLVED` y `GENERACION` son globales del proceso
    /// y las pruebas corren en paralelo, así que repartir esto en varias hace
    /// que una invalide mientras la otra comprueba lo que guardó.
    #[test]
    fn lo_memorizado_se_guarda_se_invalida_y_no_acepta_resultados_viejos() {
        let inexistente = "vasak.prueba.que.no.existe";
        invalidate_icon_cache();

        // El resultado negativo también se guarda: si no, cada evento del
        // compositor volvería a recorrer todos los directorios de aplicaciones
        // por cada ventana sin entrada.
        assert_eq!(icon_for_app_id(inexistente), None);
        assert!(
            RESOLVED
                .read()
                .expect("cerrojo envenenado")
                .contains_key(inexistente),
            "el resultado negativo no quedó memorizado"
        );

        // Invalidar vacía.
        invalidate_icon_cache();
        assert!(
            !RESOLVED
                .read()
                .expect("cerrojo envenenado")
                .contains_key(inexistente),
            "invalidar no vació lo memorizado"
        );

        // Y un resultado de antes de invalidar no se guarda. La búsqueda lee
        // archivos sin el cerrojo tomado, así que puede terminar después de que
        // alguien invalidó; guardarlo dejaría el icono viejo memorizado hasta el
        // próximo cambio en el disco.
        let generacion = GENERACION.load(Ordering::SeqCst);
        invalidate_icon_cache();
        assert!(
            !memorizar(inexistente, Some("viejo".into()), generacion),
            "se guardó un resultado de antes de invalidar"
        );
        assert!(
            !RESOLVED
                .read()
                .expect("cerrojo envenenado")
                .contains_key(inexistente)
        );

        // Con la generación al día, sí.
        let al_dia = GENERACION.load(Ordering::SeqCst);
        assert!(memorizar(inexistente, Some("nuevo".into()), al_dia));

        invalidate_icon_cache();
    }
}
