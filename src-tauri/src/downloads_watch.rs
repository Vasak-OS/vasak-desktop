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
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

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

/// Los parciales de la carpeta con su tamaño actual. Un archivo ilegible se
/// ignora, no corta la vuelta.
fn scan_partials(dir: &Path) -> Vec<(String, u64)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
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
    out
}

/// Vigila la carpeta de descargas y le avisa al panel qué descargas hay en curso
/// por el evento `downloads-update`. Sin carpeta de descargas XDG no hace nada.
pub fn watch_downloads(app: &AppHandle) {
    let app = app.clone();
    let Some(dir) = dirs::download_dir() else {
        log_info("Sin carpeta de descargas XDG; la cápsula no mostrará descargas.");
        return;
    };

    std::thread::spawn(move || {
        let dir_str = dir.to_string_lossy().to_string();
        let mut tracker = Tracker::default();
        let mut last: Vec<String> = Vec::new();
        loop {
            let scan = scan_partials(&dir);
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
