//! Vigila la carpeta del escritorio para no tener que releerla cada tanto.
//!
//! El widget de archivos releía el directorio **cada diez segundos, siempre**:
//! seis veces por minuto, con sus `stat` y la resolución del icono de cada
//! archivo, esté mirando alguien o no y haya cambiado algo o no. Y como el
//! escritorio es una ventana de capa que nunca queda «oculta» para el navegador,
//! pausar con `document.hidden` no lo habría salvado.
//!
//! Ahora el disco avisa. La relectura pasa a ocurrir cuando de verdad cambió
//! algo, que es la única vez que hace falta.

use inotify::{Inotify, WatchMask};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::inotify_rafaga::esperar_rafaga;
use crate::logger::{log_error, log_info};

/// Se emite cuando cambió el contenido de la carpeta del escritorio.
pub const DESKTOP_CHANGED_EVENT: &str = "desktop-files-changed";

/// Una descarga crea el archivo, lo va escribiendo y lo renombra al final;
/// descomprimir algo son cientos de creaciones seguidas. Se espera a que la
/// ráfaga se apague para releer una vez y no una por archivo.
const REPOSO: Duration = Duration::from_millis(400);

/// La carpeta del escritorio según `user-dirs.dirs`, dado su contenido.
///
/// Separado de la lectura del archivo para poder probarlo: el formato admite
/// comentarios, comillas y `$HOME`, y una carpeta mal resuelta deja el widget
/// vigilando un directorio que no es el que muestra —de esos errores que no dan
/// ningún síntoma más que «no se actualiza».
pub fn escritorio_en_user_dirs(contenido: &str, home: &str) -> Option<PathBuf> {
    for linea in contenido.lines() {
        let linea = linea.trim();
        if linea.starts_with('#') {
            continue;
        }

        let Some(valor) = linea.strip_prefix("XDG_DESKTOP_DIR=") else {
            continue;
        };
        let valor = valor.trim().trim_matches('"');
        if valor.is_empty() {
            continue;
        }

        return Some(PathBuf::from(valor.replace("$HOME", home)));
    }

    None
}

/// La carpeta del escritorio, o `$HOME/Desktop` si no hay configuración.
///
/// El mismo orden que usa el frontend, para que los dos miren la misma carpeta.
fn directorio_del_escritorio() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;

    let config = PathBuf::from(&home).join(".config").join("user-dirs.dirs");
    if let Ok(contenido) = std::fs::read_to_string(&config) {
        if let Some(ruta) = escritorio_en_user_dirs(&contenido, &home) {
            return Some(ruta);
        }
    }

    Some(PathBuf::from(home).join("Desktop"))
}

/// Avisa al frontend cada vez que cambia el contenido del escritorio.
pub fn watch_desktop_dir(app: &AppHandle) {
    let app = app.clone();

    std::thread::spawn(move || {
        let Some(directorio) = directorio_del_escritorio() else {
            log_error("Sin HOME no se puede ubicar la carpeta del escritorio; el widget de archivos no se actualizará solo.");
            return;
        };

        let mut inotify = match Inotify::init() {
            Ok(inotify) => inotify,
            Err(error) => {
                log_error(&format!(
                    "No se pudo iniciar inotify para el escritorio: {}. El widget de archivos no se actualizará solo.",
                    error
                ));
                return;
            }
        };

        // ATTRIB además de los cambios de contenido: un `chmod +x` cambia el
        // icono que se muestra sin crear ni borrar nada.
        let mask = WatchMask::CREATE
            | WatchMask::DELETE
            | WatchMask::MOVED_TO
            | WatchMask::MOVED_FROM
            | WatchMask::CLOSE_WRITE
            | WatchMask::ATTRIB;

        if let Err(error) = inotify.watches().add(&directorio, mask) {
            log_error(&format!(
                "Sin vigilancia sobre {}: {}. El widget de archivos no se actualizará solo.",
                directorio.display(),
                error
            ));
            return;
        }

        log_info(&format!(
            "Vigilando {} para el widget de archivos",
            directorio.display()
        ));

        let mut buffer = [0u8; 4096];

        loop {
            match esperar_rafaga(&mut inotify, &mut buffer, REPOSO) {
                Ok(()) => {
                    let _ = app.emit(DESKTOP_CHANGED_EVENT, ());
                }
                Err(error) => {
                    log_error(&format!(
                        "Vigilancia del escritorio interrumpida: {}",
                        error
                    ));
                    return;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn se_resuelve_home_y_se_sacan_las_comillas() {
        let contenido = "XDG_DOWNLOAD_DIR=\"$HOME/Descargas\"\nXDG_DESKTOP_DIR=\"$HOME/Escritorio\"\n";
        assert_eq!(
            escritorio_en_user_dirs(contenido, "/home/pato"),
            Some(PathBuf::from("/home/pato/Escritorio"))
        );
    }

    #[test]
    fn una_ruta_absoluta_se_toma_tal_cual() {
        assert_eq!(
            escritorio_en_user_dirs("XDG_DESKTOP_DIR=\"/mnt/datos/escritorio\"", "/home/pato"),
            Some(PathBuf::from("/mnt/datos/escritorio"))
        );
    }

    #[test]
    fn los_comentarios_no_cuentan() {
        // Si contara, el widget vigilaría una carpeta equivocada y no se
        // actualizaría nunca, sin ningún mensaje de error.
        let contenido = "# XDG_DESKTOP_DIR=\"$HOME/Viejo\"\nXDG_DESKTOP_DIR=\"$HOME/Escritorio\"\n";
        assert_eq!(
            escritorio_en_user_dirs(contenido, "/home/pato"),
            Some(PathBuf::from("/home/pato/Escritorio"))
        );
    }

    #[test]
    fn sin_la_clave_no_se_inventa_nada() {
        assert_eq!(
            escritorio_en_user_dirs("XDG_MUSIC_DIR=\"$HOME/Musica\"", "/home/pato"),
            None
        );
        // Y una clave vacía tampoco vale: vigilar "" no falla, no hace nada.
        assert_eq!(escritorio_en_user_dirs("XDG_DESKTOP_DIR=\"\"", "/home/pato"), None);
    }

    #[test]
    fn una_carpeta_con_acentos_o_espacios_sobrevive() {
        assert_eq!(
            escritorio_en_user_dirs("XDG_DESKTOP_DIR=\"$HOME/Mi Escritorio Ñ\"", "/home/pato"),
            Some(PathBuf::from("/home/pato/Mi Escritorio Ñ"))
        );
    }
}
