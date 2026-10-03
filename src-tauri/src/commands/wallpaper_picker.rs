//! Lo que necesita el selector rápido de fondos (vasak-desktop#133).
//!
//! # Configuración prepara, el escritorio guarda
//!
//! La pantalla de fondos de vasak-settings ya sabe dos cosas que acá hacen
//! falta: hacer miniaturas chicas de los fondos (los oficiales son de 4K y 5K, y
//! diez decodificados son medio giga) y **preparar un video** antes de usarlo de
//! fondo (bajarlo a la resolución de la pantalla, 30 fps, sin audio). Copiar eso
//! acá dejaría dos preparaciones que se separan con el tiempo, así que se le
//! pide a Configuración: `vasak-settings --wallpaper list | thumbnails |
//! prepare`, que contesta en JSON y sale sin abrir su ventana (vasak-settings,
//! `wallpaper_cli.rs`).
//!
//! La clave `desktop.wallpaper` la escribe la página con el config-manager,
//! igual que Configuración: ver `src/services/wallpaper.service.ts`.
//!
//! # Sin Configuración
//!
//! Si no está instalada o falla, el selector sigue andando: la lista queda con
//! el fondo actual, las tarjetas sin miniatura llevan el icono del tema (nunca
//! el original de 5K) y el video se aplica sin preparar —que es lo mismo que
//! hace Configuración cuando falta ffmpeg—.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::process::Command;

use crate::logger::{log_info, log_warning};
use crate::windows_apps::shell_layer::{
    hide_layer_window, layer_window_exists, layer_window_visible, show_layer_window,
};
use crate::windows_apps::wallpaper_picker::{
    create_wallpaper_picker_window, WALLPAPER_PICKER_LABEL,
};

/// El programa al que se le piden las miniaturas y la preparación.
const SETTINGS_PROGRAM: &str = "vasak-settings";

/// Los fondos en movimiento que el escritorio sabe reproducir. Los mismos que
/// acepta la pantalla de fondos de Configuración y que `DesktopView.vue` prueba
/// con `canPlayType`.
const VIDEO_EXTENSIONS: [&str; 3] = ["mp4", "webm", "ogv"];

/// Un fondo, como lo dibuja el carrusel.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WallpaperEntry {
    pub path: String,
    /// La miniatura chica, o `None` si no se pudo hacer.
    pub thumbnail: Option<String>,
    pub video: bool,
}

/// Lo que devuelve `vasak-settings --wallpaper prepare`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreparedWallpaper {
    pub path: String,
    pub optimized: bool,
    pub detail: String,
}

pub fn is_video_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(|extension| VIDEO_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Qué fondos lista el selector: los oficiales y, si no es uno de ellos, el que
/// está puesto ahora, **primero** —el carrusel abre centrado en él y así queda
/// en una punta, no perdido en el medio de los oficiales—.
///
/// Un fondo elegido antes desde Configuración (una foto propia, un video) no se
/// pierde por abrir el selector: sigue en la fila mientras sea el aplicado.
pub fn merge_catalog(official: Vec<String>, current: Option<&str>) -> Vec<String> {
    let mut paths: Vec<String> = Vec::with_capacity(official.len() + 1);

    if let Some(current) = current.map(str::trim).filter(|current| !current.is_empty()) {
        if !official.iter().any(|path| path == current) {
            paths.push(current.to_string());
        }
    }

    for path in official {
        if !paths.contains(&path) {
            paths.push(path);
        }
    }

    paths
}

/// Arma la fila con las miniaturas que devolvió Configuración.
pub fn build_entries(
    paths: Vec<String>,
    thumbnails: &HashMap<String, Option<String>>,
) -> Vec<WallpaperEntry> {
    paths
        .into_iter()
        .map(|path| WallpaperEntry {
            thumbnail: thumbnails.get(&path).cloned().flatten(),
            video: is_video_path(&path),
            path,
        })
        .collect()
}

/// Corre `vasak-settings --wallpaper …` y lee el JSON que deja en la salida.
///
/// Lo usan el selector y «Seguir al fondo» (`wallpaper_colors.rs`): una sola
/// forma de pedirle algo a Configuración.
///
/// Desde el hogar, como toda aplicación que abre el escritorio (ver
/// `runner.rs`): con otro directorio de trabajo Configuración podría leer
/// catálogos de textos ajenos.
pub(super) async fn ask_settings<T: serde::de::DeserializeOwned>(
    args: &[&str],
) -> Result<T, String> {
    let output = Command::new(SETTINGS_PROGRAM)
        .arg("--wallpaper")
        .args(args)
        .current_dir(dirs::home_dir().unwrap_or_else(|| "/".into()))
        .output()
        .await
        .map_err(|error| format!("no se pudo ejecutar {SETTINGS_PROGRAM}: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("{SETTINGS_PROGRAM} contestó algo que no se entiende: {error}"))
}

/// La fila del carrusel: los fondos oficiales y el actual, con sus miniaturas.
#[tauri::command]
pub async fn wallpaper_catalog(current: Option<String>) -> Result<Vec<WallpaperEntry>, String> {
    let official = ask_settings::<Vec<String>>(&["list"])
        .await
        .unwrap_or_else(|error| {
            log_warning(&format!(
                "[wallpaper_picker] sin la lista de fondos oficiales: {error}"
            ));
            Vec::new()
        });

    let paths = merge_catalog(official, current.as_deref());
    if paths.is_empty() {
        return Ok(Vec::new());
    }

    let mut args: Vec<&str> = vec!["thumbnails"];
    args.extend(paths.iter().map(String::as_str));
    let thumbnails = ask_settings::<HashMap<String, Option<String>>>(&args)
        .await
        .unwrap_or_else(|error| {
            log_warning(&format!("[wallpaper_picker] sin miniaturas: {error}"));
            HashMap::new()
        });

    Ok(build_entries(paths, &thumbnails))
}

/// Prepara un video para usarlo de fondo, con la misma preparación que
/// Configuración. Una imagen vuelve igual, sin preguntarle a nadie.
#[tauri::command]
pub async fn prepare_wallpaper(path: String) -> Result<PreparedWallpaper, String> {
    if !is_video_path(&path) {
        return Ok(PreparedWallpaper {
            path,
            optimized: false,
            detail: String::new(),
        });
    }

    match ask_settings::<PreparedWallpaper>(&["prepare", &path]).await {
        Ok(prepared) => {
            log_info(&format!(
                "[wallpaper_picker] {} preparado: {}",
                path, prepared.detail
            ));
            Ok(prepared)
        }
        // Que no se pueda preparar no impide poner el fondo, igual que en
        // Configuración: se usa el original.
        Err(error) => {
            log_warning(&format!(
                "[wallpaper_picker] no se pudo preparar {path}, se usa el original: {error}"
            ));
            Ok(PreparedWallpaper {
                detail: error,
                path,
                optimized: false,
            })
        }
    }
}

/// Abre o cierra el selector. Lo llaman el menú del escritorio y D-Bus
/// (`OpenWallpaperPicker`).
#[tauri::command]
pub fn toggle_wallpaper_picker(app: AppHandle) -> Result<(), String> {
    let handle = app.clone();
    // El registro de superficies vive en el hilo principal: desde otro hilo se
    // ve vacío y la superficie se crearía dos veces.
    app.run_on_main_thread(move || {
        if !layer_window_exists(WALLPAPER_PICKER_LABEL) {
            log_info("[wallpaper_picker] armando la superficie");
            if let Err(error) = create_wallpaper_picker_window(&handle) {
                log_warning(&format!("[wallpaper_picker] no se pudo crear: {error}"));
            }
            return;
        }

        if layer_window_visible(WALLPAPER_PICKER_LABEL).unwrap_or(false) {
            hide_layer_window(WALLPAPER_PICKER_LABEL);
            return;
        }

        // La página vuelve a pedir la lista y a centrarse en el fondo actual:
        // la superficie se esconde, no se destruye, así que Vue no se vuelve a
        // montar.
        if let Some(webview) = handle.get_webview_window(WALLPAPER_PICKER_LABEL) {
            let _ = webview.emit("window-shown", ());
        }
        show_layer_window(WALLPAPER_PICKER_LABEL);
    })
    .map_err(|error| error.to_string())
}

/// Lo esconde sin preguntar si está abierto: la página se cierra con esto, por
/// lo mismo que el centro de control (ver `hide_control_center`).
#[tauri::command]
pub fn hide_wallpaper_picker() -> Result<(), ()> {
    hide_layer_window(WALLPAPER_PICKER_LABEL);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(list: &[&str]) -> Vec<String> {
        list.iter().map(|path| path.to_string()).collect()
    }

    #[test]
    fn el_fondo_actual_va_primero_si_no_es_oficial() {
        let merged = merge_catalog(paths(&["/o/1.jpg", "/o/2.jpg"]), Some("/home/p/video.mp4"));
        assert_eq!(
            merged,
            paths(&["/home/p/video.mp4", "/o/1.jpg", "/o/2.jpg"])
        );
    }

    #[test]
    fn un_fondo_oficial_no_se_repite() {
        let merged = merge_catalog(paths(&["/o/1.jpg", "/o/2.jpg"]), Some("/o/2.jpg"));
        assert_eq!(merged, paths(&["/o/1.jpg", "/o/2.jpg"]));
    }

    #[test]
    fn sin_configuracion_queda_al_menos_el_actual() {
        assert_eq!(
            merge_catalog(Vec::new(), Some("/home/p/foto.png")),
            paths(&["/home/p/foto.png"])
        );
        assert!(merge_catalog(Vec::new(), None).is_empty());
        assert!(
            merge_catalog(Vec::new(), Some("  ")).is_empty(),
            "un fondo vacío no es un fondo"
        );
    }

    #[test]
    fn reconoce_los_videos_que_el_escritorio_reproduce() {
        assert!(is_video_path("/a/b.mp4"));
        assert!(is_video_path("/a/b.WEBM"));
        assert!(is_video_path("/a/b.ogv"));
        // Un mkv no lo abre WebKit: Configuración tampoco lo acepta.
        assert!(!is_video_path("/a/b.mkv"));
        assert!(!is_video_path("/a/b.jpg"));
        assert!(!is_video_path("/a/sin-extension"));
    }

    #[test]
    fn la_miniatura_que_falta_queda_vacia_y_no_saca_el_fondo() {
        let mut thumbnails = HashMap::new();
        thumbnails.insert("/o/1.jpg".to_string(), Some("/c/m1.jpg".to_string()));
        thumbnails.insert("/o/2.mp4".to_string(), None);

        let entries = build_entries(paths(&["/o/1.jpg", "/o/2.mp4", "/o/3.jpg"]), &thumbnails);

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].thumbnail.as_deref(), Some("/c/m1.jpg"));
        assert_eq!(entries[1].thumbnail, None);
        assert!(entries[1].video);
        assert_eq!(entries[2].thumbnail, None, "sin respuesta de Configuración");
    }

    #[test]
    fn la_respuesta_de_prepare_se_lee() {
        let prepared: PreparedWallpaper = serde_json::from_str(
            r#"{"path":"/c/fondo-1.mp4","optimized":true,"detail":"1920×1080, 30 fps, sin audio"}"#,
        )
        .unwrap();
        assert_eq!(prepared.path, "/c/fondo-1.mp4");
        assert!(prepared.optimized);
    }

    #[test]
    fn una_imagen_no_se_prepara() {
        let prepared =
            tauri::async_runtime::block_on(prepare_wallpaper("/o/1.jpg".into())).unwrap();
        assert_eq!(prepared.path, "/o/1.jpg");
        assert!(!prepared.optimized);
    }
}
