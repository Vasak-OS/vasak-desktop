//! Darle acceso del protocolo de assets a un archivo de fondo puntual.
//!
//! # Por qué no alcanza con el alcance declarado
//!
//! `tauri.conf.json` declara un alcance (`assetProtocol.scope`) con globs: el
//! hogar, las carpetas del sistema, la caché de miniaturas. Pero un glob no
//! cubre tres casos que los fondos sí necesitan:
//!
//!  · **Una subcarpeta oculta.** Tauri obliga a `require_literal_leading_dot`
//!    en Unix, así que `$HOME/**` **no** alcanza `~/.cache/…` ni ninguna
//!    carpeta que empiece con punto. (Por eso las miniaturas, que viven en
//!    `~/.cache/vasak/wallpapers`, llevan además su propio patrón en el
//!    alcance: ver vasak-desktop#162.)
//!  · **Una carpeta propia fuera del hogar** (vasak-settings#148): un fondo en
//!    `/mnt/fotos` no lo prevé ningún glob del archivo.
//!  · **Una ruta con enlaces simbólicos o `..`.** El protocolo canonicaliza la
//!    ruta pedida antes de compararla; si no coincide con el patrón declarado,
//!    la rechaza y el fondo se ve negro con el signo de interrogación
//!    (vasak-desktop#163).
//!
//! Autorizar el archivo **exacto y ya canonicalizado** en tiempo de ejecución
//! no depende de ninguno de esos bordes: `allow_file` lo suma al alcance tal
//! cual, con su carpeta canónica, y el protocolo lo sirve.
//!
//! Es el mismo comando en el escritorio y en Configuración: un solo contrato
//! para que los dos muestren cualquier fondo igual.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

/// Resuelve la ruta a su forma canónica —sin enlaces simbólicos, sin `..`, y
/// absoluta—, que es la que hay que autorizar y la que hay que pasarle después
/// a `convertFileSrc`. Separada del comando para poder probarla.
///
/// Canonicalizar exige que el archivo exista, que es justo lo que queremos: un
/// fondo que no está no es un fondo, y autorizar una ruta inexistente no
/// serviría de nada.
pub fn canonical_wallpaper_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("la ruta del fondo está vacía".into());
    }

    std::fs::canonicalize(trimmed)
        .map_err(|error| format!("no se pudo resolver {trimmed}: {error}"))
}

/// Autoriza un archivo de fondo (imagen, video o miniatura) en el protocolo de
/// assets y devuelve su ruta canónica, la que hay que usar con
/// `convertFileSrc`.
#[tauri::command]
pub fn allow_wallpaper_asset(app: AppHandle, path: String) -> Result<String, String> {
    let canonical = canonical_wallpaper_path(&path)?;
    app.asset_protocol_scope()
        .allow_file(&canonical)
        .map_err(|error| format!("no se pudo autorizar {}: {error}", canonical.display()))?;
    Ok(canonical.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn una_ruta_vacia_es_un_error() {
        assert!(canonical_wallpaper_path("").is_err());
        assert!(canonical_wallpaper_path("   ").is_err());
    }

    #[test]
    fn una_ruta_que_no_existe_es_un_error() {
        // No se puede autorizar lo que no está, y canonicalizar lo deja claro
        // antes de tocar el alcance.
        assert!(canonical_wallpaper_path("/no/existe/fondo.jpg").is_err());
    }

    #[test]
    fn resuelve_los_enlaces_simbolicos_a_la_ruta_real() {
        let dir = std::env::temp_dir().join("vasak-desktop-assets-prueba");
        let _ = std::fs::create_dir_all(&dir);

        let real = dir.join("fondo-real.jpg");
        std::fs::write(&real, vec![0u8; 16]).unwrap();

        let link = dir.join("fondo-enlace.jpg");
        let _ = std::fs::remove_file(&link);
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&real, &link).unwrap();
            let resolved = canonical_wallpaper_path(link.to_str().unwrap()).unwrap();
            let real_canonical = std::fs::canonicalize(&real).unwrap();
            assert_eq!(
                resolved, real_canonical,
                "el enlace tiene que resolver al archivo real"
            );
        }
    }

    #[test]
    fn saca_el_punto_y_los_dos_puntos_de_la_ruta() {
        let dir = std::env::temp_dir().join("vasak-desktop-assets-prueba-2");
        let sub = dir.join("sub");
        let _ = std::fs::create_dir_all(&sub);

        let real = dir.join("fondo.jpg");
        std::fs::write(&real, vec![0u8; 16]).unwrap();

        // dir/sub/../fondo.jpg → dir/fondo.jpg
        let noisy = sub.join("..").join("fondo.jpg");
        let resolved = canonical_wallpaper_path(noisy.to_str().unwrap()).unwrap();
        assert_eq!(resolved, std::fs::canonicalize(&real).unwrap());
    }
}
