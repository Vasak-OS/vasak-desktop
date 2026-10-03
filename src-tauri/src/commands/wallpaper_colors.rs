//! Los píxeles del fondo de pantalla para «Seguir al fondo»
//! (Vasak-OS/vasak-settings#134).
//!
//! El escritorio no lee el fondo por su cuenta: se lo pide a Configuración,
//! `vasak-settings --wallpaper pixels RUTA`, por el mismo camino que el
//! selector de fondos (`wallpaper_picker::ask_settings`). Es la misma muestra
//! que lee la pantalla de Apariencia —el cuadro de la miniatura, en RGB crudo
//! de 96 px de ancho—, así que la lectura del fondo es una sola y el mismo fondo
//! da los mismos colores lo saque quien lo saque.
//!
//! Los colores los calcula la página con `followWallpaper` del config-manager.

use serde::{Deserialize, Serialize};

use super::wallpaper_picker::ask_settings;

/// Lo que devuelve `vasak-settings --wallpaper pixels`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallpaperPixels {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    #[serde(default)]
    pub video: bool,
}

/// Comprueba que la muestra esté entera.
///
/// Una muestra más corta que lo que dice medir es un fondo que no se leyó: se
/// rechaza acá para que nadie saque colores de una parte.
pub fn check_pixels(pixels: WallpaperPixels) -> Result<WallpaperPixels, String> {
    let expected = pixels.width as usize * pixels.height as usize * 3;
    if expected == 0 || pixels.data.len() < expected {
        return Err("la muestra del fondo llegó vacía o cortada".into());
    }
    Ok(pixels)
}

/// Los píxeles del fondo. Un error deja los colores como estaban.
#[tauri::command]
pub async fn wallpaper_pixels(path: String) -> Result<WallpaperPixels, String> {
    check_pixels(ask_settings::<WallpaperPixels>(&["pixels", &path]).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixels(width: u32, height: u32, data: Vec<u8>) -> WallpaperPixels {
        WallpaperPixels {
            width,
            height,
            data,
            video: false,
        }
    }

    #[test]
    fn lee_la_muestra_que_devuelve_configuracion() {
        let json = serde_json::json!({
            "width": 2, "height": 1, "data": [1, 2, 3, 4, 5, 6], "video": true
        });
        let parsed: WallpaperPixels = serde_json::from_value(json).unwrap();
        let checked = check_pixels(parsed).unwrap();
        assert_eq!(checked.width, 2);
        assert_eq!(checked.data, vec![1, 2, 3, 4, 5, 6]);
        assert!(checked.video);
    }

    #[test]
    fn una_muestra_cortada_o_vacia_es_un_error() {
        assert!(check_pixels(pixels(2, 2, vec![1, 2, 3])).is_err());
        assert!(check_pixels(pixels(0, 0, vec![])).is_err());
    }

    #[test]
    fn una_muestra_entera_pasa() {
        assert!(check_pixels(pixels(1, 1, vec![9, 9, 9])).is_ok());
    }
}
