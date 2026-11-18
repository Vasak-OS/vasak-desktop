//! Las descargas del navegador en la cápsula de actividad del panel.
//!
//! Los navegadores no publican su avance por `com.canonical.Unity.LauncherEntry`
//! (Chrome quitó su integración de Unity, Firefox nunca la trajo nativa), así que
//! no hay nada que escuchar en el bus. Lo que sí dejan es un archivo parcial en la
//! carpeta de descargas mientras bajan: `*.crdownload` (Chrome/Chromium), `*.part`
//! y `*.partial` (Firefox). Se vigila esa carpeta y cada parcial en curso es una
//! actividad más.
//!
//! No hay forma de saber el total desde el parcial (no trae el `Content-Length`),
//! así que el avance es **indeterminado**: se muestra que hay una descarga, no el
//! porcentaje (decisión 2: nunca roto). Lo que sí se puede ver es si el archivo
//! **sigue creciendo**: una descarga activa crece, una cancelada o terminada deja
//! de crecer. Esa es la regla —mostrar los parciales que crecieron hace poco—, y
//! de un saque resuelve el fin, la cancelación y el parcial viejo colgado sin
//! necesidad de inotify ni de un cronómetro aparte.
//!
//! shortcut: una descarga **en pausa** (no cancelada) deja de crecer y se oculta;
//! reaparece al reanudarse. Subir a leer el estado real del navegador si molesta.

use crate::logger::{log_info, log_warning};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Cada cuánto se mira la carpeta.
const POLL: Duration = Duration::from_secs(1);
/// Cuánto sigue mostrándose un parcial desde la última vez que creció. Absorbe el
/// buffer del navegador (escribe a saltos) y da el margen para sacar una descarga
/// cancelada o terminada.
const GRACE: Duration = Duration::from_secs(3);

/// Los sufijos de un archivo de descarga a medio bajar.
const PARTIAL_SUFFIXES: &[&str] = &[".crdownload", ".part", ".partial"];

/// Una descarga en curso, para la cápsula de actividad.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DownloadActivity {
    /// El nombre del archivo parcial, estable mientras dura la descarga.
    pub id: String,
    /// El nombre final, sin el sufijo del parcial: `ubuntu.iso.crdownload` → `ubuntu.iso`.
    pub title: String,
    /// La carpeta de descargas, para abrirla al tocar la actividad.
    pub dir: String,
}

/// El nombre que se muestra: el del archivo sin el sufijo del parcial, o `None`
/// si el nombre no es el de una descarga a medio bajar.
pub fn partial_title(name: &str) -> Option<String> {
    for suffix in PARTIAL_SUFFIXES {
        if let Some(base) = name.strip_suffix(suffix) {
            if !base.is_empty() {
                return Some(base.to_string());
            }
        }
    }
    None
}

/// Lo que se sabe de los parciales de una vuelta a la otra: el último tamaño y la
/// última vez que creció.
#[derive(Default)]
pub struct Tracker {
    seen: HashMap<String, (u64, Option<Instant>)>,
}

impl Tracker {
    /// Los nombres de los parciales que hay que mostrar ahora, ordenados: los que
    /// crecieron hace menos de `grace`. Un parcial recién aparecido todavía no
    /// creció (no se lo mostró antes), así que no cuenta hasta que se lo vea
    /// crecer: eso deja fuera a los parciales viejos y colgados.
    pub fn active(&mut self, scan: &[(String, u64)], now: Instant, grace: Duration) -> Vec<String> {
        let current: HashSet<&str> = scan.iter().map(|(name, _)| name.as_str()).collect();
        self.seen.retain(|name, _| current.contains(name.as_str()));

        let mut out = Vec::new();
        for (name, size) in scan {
            let entry = self.seen.entry(name.clone()).or_insert((*size, None));
            if *size > entry.0 {
                entry.1 = Some(now);
            }
            entry.0 = *size;
            if entry
                .1
                .is_some_and(|grew| now.duration_since(grew) <= grace)
            {
                out.push(name.clone());
            }
        }
        out.sort();
        out
    }
}

/// Los parciales de la carpeta con su tamaño actual, o `None` si la carpeta no
/// se pudo leer. Distinguir el fallo del vacío importa: un `read_dir` que falla
/// una vuelta (la carpeta desapareció un instante, un error pasajero) no es «no
/// hay descargas»; vaciar el seguimiento ahí ocultaría una descarga en curso y
/// la trataría como nueva al volver. Un archivo suelto ilegible sí se ignora.
fn scan_partials(dir: &Path) -> Option<Vec<(String, u64)>> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if partial_title(&name).is_none() {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                out.push((name, meta.len()));
            }
        }
    }
    Some(out)
}

/// La última lista de descargas emitida, para que una vista que recién montó la
/// pida de entrada: el bus de eventos no reenvía lo que ya pasó, así que una
/// descarga que ya estaba en curso cuando se (re)abrió el panel no se vería
/// hasta el próximo cambio. La misma idea que `get_windows`/`get_launcher_entries`.
#[derive(Default)]
pub struct ActiveDownloads(pub Mutex<Vec<DownloadActivity>>);

/// Las descargas en curso ahora mismo. La vista la llama al montar; después se
/// mantiene sola por el evento `downloads-update`.
#[tauri::command]
pub fn active_downloads(state: tauri::State<'_, ActiveDownloads>) -> Vec<DownloadActivity> {
    state.0.lock().map(|held| held.clone()).unwrap_or_default()
}

/// Vigila la carpeta de descargas y le avisa al panel qué descargas hay en curso
/// por el evento `downloads-update`. Sin carpeta de descargas XDG no hace nada.
pub fn watch_downloads(app: &AppHandle) {
    let app = app.clone();
    // El estado que lee el comando `active_downloads`, siempre presente —aunque no
    // haya carpeta de descargas queda vacío— para que el comando no falle.
    app.manage(ActiveDownloads::default());
    let Some(dir) = dirs::download_dir() else {
        log_info("Sin carpeta de descargas XDG; la cápsula no mostrará descargas.");
        return;
    };

    std::thread::spawn(move || {
        let dir_str = dir.to_string_lossy().to_string();
        let mut tracker = Tracker::default();
        let mut last: Vec<String> = Vec::new();
        loop {
            // Un `read_dir` fallido no toca el seguimiento: se saltea la vuelta.
            let Some(scan) = scan_partials(&dir) else {
                std::thread::sleep(POLL);
                continue;
            };
            let active = tracker.active(&scan, Instant::now(), GRACE);
            if active != last {
                let payload: Vec<DownloadActivity> = active
                    .iter()
                    .map(|name| DownloadActivity {
                        id: name.clone(),
                        title: partial_title(name).unwrap_or_else(|| name.clone()),
                        dir: dir_str.clone(),
                    })
                    .collect();
                if let Some(state) = app.try_state::<ActiveDownloads>() {
                    if let Ok(mut held) = state.0.lock() {
                        *held = payload.clone();
                    }
                }
                if let Err(error) = app.emit("downloads-update", payload) {
                    log_warning(&format!(
                        "[Downloads] No se pudo avisar a la vista: {error}"
                    ));
                }
                last = active;
            }
            std::thread::sleep(POLL);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRACE_T: Duration = Duration::from_secs(3);

    fn scan(items: &[(&str, u64)]) -> Vec<(String, u64)> {
        items.iter().map(|(n, s)| (n.to_string(), *s)).collect()
    }

    #[test]
    fn el_titulo_pierde_el_sufijo_del_parcial() {
        assert_eq!(
            partial_title("ubuntu.iso.crdownload").as_deref(),
            Some("ubuntu.iso")
        );
        assert_eq!(
            partial_title("video.mkv.part").as_deref(),
            Some("video.mkv")
        );
        assert_eq!(
            partial_title("foto.png.partial").as_deref(),
            Some("foto.png")
        );
    }

    #[test]
    fn un_nombre_sin_sufijo_o_con_base_vacia_no_es_un_parcial() {
        assert_eq!(partial_title("ubuntu.iso"), None);
        assert_eq!(partial_title(".part"), None);
        assert_eq!(partial_title(".crdownload"), None);
        assert_eq!(partial_title("notas.partido"), None);
    }

    #[test]
    fn un_parcial_recien_visto_no_cuenta_hasta_verlo_crecer() {
        let base = Instant::now();
        let mut tracker = Tracker::default();
        let primera = scan(&[("viejo.iso.part", 100)]);
        assert!(tracker.active(&primera, base, GRACE_T).is_empty());
        // Mismo tamaño en la vuelta siguiente: sigue colgado.
        assert!(tracker
            .active(&primera, base + Duration::from_secs(1), GRACE_T)
            .is_empty());
    }

    #[test]
    fn un_parcial_que_crece_esta_activo() {
        let base = Instant::now();
        let mut tracker = Tracker::default();
        tracker.active(&scan(&[("a.iso.part", 100)]), base, GRACE_T);
        let activos = tracker.active(
            &scan(&[("a.iso.part", 200)]),
            base + Duration::from_secs(1),
            GRACE_T,
        );
        assert_eq!(activos, vec!["a.iso.part".to_string()]);
    }

    #[test]
    fn deja_de_mostrarse_cuando_pasa_la_gracia_sin_crecer() {
        let base = Instant::now();
        let mut tracker = Tracker::default();
        tracker.active(&scan(&[("a.part", 1)]), base, GRACE_T);
        let creció = base + Duration::from_secs(1);
        assert_eq!(
            tracker
                .active(&scan(&[("a.part", 2)]), creció, GRACE_T)
                .len(),
            1
        );
        // Justo en el límite todavía cuenta.
        let limite = creció + GRACE_T;
        assert_eq!(
            tracker
                .active(&scan(&[("a.part", 2)]), limite, GRACE_T)
                .len(),
            1
        );
        // Un instante después se cae.
        let tarde = limite + Duration::from_millis(1);
        assert!(tracker
            .active(&scan(&[("a.part", 2)]), tarde, GRACE_T)
            .is_empty());
    }

    #[test]
    fn un_parcial_que_desaparece_se_olvida_y_no_reaparece_como_crecido() {
        let base = Instant::now();
        let mut tracker = Tracker::default();
        tracker.active(&scan(&[("a.part", 1)]), base, GRACE_T);
        let t1 = base + Duration::from_secs(1);
        assert_eq!(
            tracker.active(&scan(&[("a.part", 2)]), t1, GRACE_T).len(),
            1
        );
        // Desaparece (terminó o se canceló).
        let t2 = base + Duration::from_secs(2);
        assert!(tracker.active(&[], t2, GRACE_T).is_empty());
        // Vuelve con el mismo tamaño: es nuevo, no cuenta como crecido.
        let t3 = base + Duration::from_secs(3);
        assert!(tracker
            .active(&scan(&[("a.part", 2)]), t3, GRACE_T)
            .is_empty());
    }

    #[test]
    fn la_salida_va_ordenada_alfabeticamente() {
        let base = Instant::now();
        let mut tracker = Tracker::default();
        tracker.active(
            &scan(&[("z.part", 1), ("a.part", 1), ("m.part", 1)]),
            base,
            GRACE_T,
        );
        let activos = tracker.active(
            &scan(&[("z.part", 2), ("a.part", 2), ("m.part", 2)]),
            base + Duration::from_secs(1),
            GRACE_T,
        );
        assert_eq!(activos, vec!["a.part", "m.part", "z.part"]);
    }

    #[test]
    fn cada_parcial_se_juzga_por_su_propio_crecimiento() {
        let base = Instant::now();
        let mut tracker = Tracker::default();
        tracker.active(&scan(&[("a.part", 1), ("b.part", 1)]), base, GRACE_T);
        let activos = tracker.active(
            &scan(&[("a.part", 2), ("b.part", 1)]),
            base + Duration::from_secs(1),
            GRACE_T,
        );
        assert_eq!(activos, vec!["a.part".to_string()]);
    }

    /// Carpeta temporal única que se borra sola, sin sumar dependencias.
    struct Carpeta(std::path::PathBuf);
    impl Carpeta {
        fn nueva(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "vasak-downloads-{tag}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Carpeta(dir)
        }
    }
    impl Drop for Carpeta {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn el_escaneo_devuelve_solo_los_parciales_con_su_tamano() {
        let carpeta = Carpeta::nueva("scan");
        std::fs::write(carpeta.0.join("a.iso.crdownload"), [0u8; 10]).unwrap();
        std::fs::write(carpeta.0.join("b.mkv.part"), [0u8; 20]).unwrap();
        std::fs::write(carpeta.0.join("c.png.partial"), [0u8; 30]).unwrap();
        std::fs::write(carpeta.0.join("terminado.iso"), [0u8; 99]).unwrap();
        // Un directorio con sufijo de parcial no es una descarga.
        std::fs::create_dir(carpeta.0.join("carpeta.part")).unwrap();

        let mut resultado = scan_partials(&carpeta.0).expect("la carpeta existe");
        resultado.sort();
        assert_eq!(
            resultado,
            scan(&[
                ("a.iso.crdownload", 10),
                ("b.mkv.part", 20),
                ("c.png.partial", 30)
            ])
        );
    }

    #[test]
    fn el_escaneo_de_una_carpeta_ilegible_es_none_y_no_vacio() {
        let carpeta = Carpeta::nueva("falta");
        assert_eq!(scan_partials(&carpeta.0.join("no-existe")), None);
        // Una carpeta legible sin parciales sí es «vacío», distinto del fallo.
        assert_eq!(scan_partials(&carpeta.0), Some(vec![]));
    }
}
