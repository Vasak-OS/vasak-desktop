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
//! # Pero no cualquier archivo
//!
//! Autorizar la ruta exacta que pide la página sería abrir una puerta: una
//! página podría pedir `~/.ssh/id_rsa` o cualquier archivo del hogar y el
//! protocolo se lo serviría. Así que antes de autorizar se comprueba que la ruta
//! canónica sea **de verdad un fondo**: uno de los oficiales, una miniatura o un
//! video preparado de la caché, un archivo de la carpeta propia configurada, o
//! el fondo aplicado ahora. Cualquier otra cosa se rechaza.
//!
//! Es el mismo comando y la misma validación en el escritorio y en
//! Configuración: un solo contrato para que los dos muestren cualquier fondo
//! legítimo igual, y ninguno sirva de más.

use std::path::{Path, PathBuf};

use serde_json::Value;
use tauri::{AppHandle, Manager};

/// Dónde viven los fondos que trae el sistema.
const OFFICIAL_WALLPAPERS_DIR: &str = "/usr/share/backgrounds/vasakos";

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

/// Las carpetas y rutas donde un fondo es legítimo.
///
/// Se arma aparte del comando para poder probar la decisión sin leer el entorno
/// ni tocar el protocolo de assets. Todas vienen ya canonicalizadas, para
/// compararlas contra la ruta pedida —también canónica— sin que un enlace
/// simbólico del medio las haga diferir.
#[derive(Debug, Default)]
pub struct WallpaperRoots {
    /// Los fondos oficiales.
    pub official: Option<PathBuf>,
    /// La caché de miniaturas y videos preparados.
    pub cache: Option<PathBuf>,
    /// La carpeta propia configurada (vasak-settings#148).
    pub custom_folder: Option<PathBuf>,
    /// El fondo aplicado ahora, que puede estar en cualquier lado.
    pub applied: Option<PathBuf>,
}

/// Si una ruta canónica es un fondo que se puede autorizar.
///
/// Sólo: los fondos oficiales, la caché de miniaturas y videos preparados, la
/// carpeta propia, o el fondo aplicado. Cualquier otra cosa se rechaza, para que
/// una página no pueda pedir un archivo privado por el protocolo de assets.
pub fn is_wallpaper_asset(canonical: &Path, roots: &WallpaperRoots) -> bool {
    let under = |root: &Option<PathBuf>| root.as_deref().is_some_and(|r| canonical.starts_with(r));

    under(&roots.official)
        || under(&roots.cache)
        || under(&roots.custom_folder)
        || roots
            .applied
            .as_deref()
            .is_some_and(|applied| canonical == applied)
}

/// Dónde vive la configuración, igual que el gestor de configuración y el resto
/// del escritorio: `VASAK_CONFIG_PATH` si está, si no `~/.config/vasak/vasak.conf`.
fn vasak_config_path() -> Option<PathBuf> {
    if let Some(set) = std::env::var_os("VASAK_CONFIG_PATH") {
        if !set.is_empty() {
            return Some(PathBuf::from(set));
        }
    }
    dirs::home_dir().map(|home| home.join(".config/vasak/vasak.conf"))
}

/// La carpeta propia y el fondo aplicado que dice `vasak.conf`, tal cual (sin
/// canonicalizar todavía). Sin archivo o sin las claves, `None`.
fn configured_locations() -> (Option<String>, Option<String>) {
    let Some(path) = vasak_config_path() else {
        return (None, None);
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return (None, None);
    };
    let Ok(value) = serde_json::from_str::<Value>(&content) else {
        return (None, None);
    };

    let desktop = value.get("desktop");
    let folder = desktop
        .and_then(|d| d.get("wallpaperfolder"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|folder| !folder.is_empty())
        .map(str::to_string);
    let applied = desktop
        .and_then(|d| d.get("wallpaper"))
        .and_then(Value::as_array)
        .and_then(|list| list.first())
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|applied| !applied.is_empty())
        .map(str::to_string);

    (folder, applied)
}

/// Canonicaliza una ruta sólo si existe; si no, la deja pasar sin resolver, que
/// para comparar con `starts_with` o `==` da igual que no coincida.
fn canonicalize_if_present(path: Option<PathBuf>) -> Option<PathBuf> {
    path.and_then(|path| std::fs::canonicalize(path).ok())
}

/// Las carpetas donde un fondo es legítimo, ya canonicalizadas, leídas del
/// entorno y de `vasak.conf`.
fn wallpaper_roots() -> WallpaperRoots {
    let (folder, applied) = configured_locations();

    WallpaperRoots {
        official: canonicalize_if_present(Some(PathBuf::from(OFFICIAL_WALLPAPERS_DIR))),
        cache: canonicalize_if_present(
            dirs::cache_dir().map(|c| c.join("vasak").join("wallpapers")),
        ),
        custom_folder: canonicalize_if_present(folder.map(PathBuf::from)),
        applied: canonicalize_if_present(applied.map(PathBuf::from)),
    }
}

/// Autoriza un archivo de fondo (imagen, video o miniatura) en el protocolo de
/// assets y devuelve su ruta canónica, la que hay que usar con
/// `convertFileSrc`. Rechaza lo que no sea un fondo legítimo.
#[tauri::command]
pub fn allow_wallpaper_asset(app: AppHandle, path: String) -> Result<String, String> {
    let canonical = canonical_wallpaper_path(&path)?;

    if !is_wallpaper_asset(&canonical, &wallpaper_roots()) {
        return Err(format!(
            "{} no es un fondo de pantalla permitido",
            canonical.display()
        ));
    }

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

    fn roots(base: &Path) -> WallpaperRoots {
        WallpaperRoots {
            official: Some(base.join("oficiales")),
            cache: Some(base.join("cache")),
            custom_folder: Some(base.join("propia")),
            applied: Some(base.join("otra").join("aplicado.jpg")),
        }
    }

    #[test]
    fn un_fondo_oficial_una_miniatura_y_la_carpeta_propia_se_permiten() {
        let base = Path::new("/var/empty-base");
        let r = roots(base);
        assert!(is_wallpaper_asset(&base.join("oficiales/w1.jpg"), &r));
        assert!(is_wallpaper_asset(&base.join("cache/miniatura-x.jpg"), &r));
        assert!(is_wallpaper_asset(&base.join("propia/sub/foto.png"), &r));
        // Nota: una subcarpeta de la carpeta propia también entra, que es lo que
        // queremos: la carpeta propia es un árbol de fondos.
    }

    #[test]
    fn el_fondo_aplicado_se_permite_exacto_pero_no_su_carpeta() {
        let base = Path::new("/var/empty-base");
        let r = roots(base);
        assert!(is_wallpaper_asset(&base.join("otra/aplicado.jpg"), &r));
        // Un vecino del aplicado no se permite: el aplicado es un archivo, no una
        // carpeta abierta.
        assert!(!is_wallpaper_asset(&base.join("otra/secreto.jpg"), &r));
    }

    #[test]
    fn un_archivo_privado_se_rechaza() {
        let base = Path::new("/var/empty-base");
        let r = roots(base);
        assert!(!is_wallpaper_asset(Path::new("/home/pato/.ssh/id_rsa"), &r));
        assert!(!is_wallpaper_asset(Path::new("/etc/shadow"), &r));
        assert!(!is_wallpaper_asset(&base.join("otracosa/foto.jpg"), &r));
    }

    #[test]
    fn sin_carpetas_no_se_permite_nada() {
        let r = WallpaperRoots::default();
        assert!(!is_wallpaper_asset(
            Path::new("/usr/share/backgrounds/vasakos/w.jpg"),
            &r
        ));
    }
}
