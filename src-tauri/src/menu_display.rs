//! El modo de tamaño del menú de inicio (vasak-desktop#210).
//!
//! Sale de `menu.displayMode` en `~/.config/vasak/vasak.conf`, la misma
//! configuración que ya lee el panel para su posición. Tres valores:
//!
//! - `normal`: el de siempre, anclado al botón (el tamaño del [`AppletSpec`]).
//! - `compact`: un menú más chico.
//! - `full`: un overlay a pantalla completa.
//!
//! Se lee igual de tolerante que `panel_position`: un valor que no sea de los
//! tres —o la clave ausente— cae en `normal`. Se lee de forma síncrona (el
//! `read_config` del gestor es `async` y esto corre al abrir el menú, en el hilo
//! de GTK), reusando `config_path` de `panel_position`.
//!
//! Con qué tamaño y anclaje abre lo decide [`applet_sizing`], que el conmutador
//! del menú le pasa al applet: `normal` cuelga del botón con el tamaño del
//! `AppletSpec`, `compact` con uno chico, y `full` es un overlay a pantalla
//! completa —los cuatro bordes, sin márgenes, sin la animación que crece desde el
//! botón—, que resuelve `anchored_applet::place_fullscreen`.

use crate::panel_position::config_path;
use crate::windows_apps::anchored_applet::AppletSizing;

/// Con qué tamaño abre el menú.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuDisplayMode {
    /// Anclado al botón, el tamaño del `AppletSpec` (900×620).
    #[default]
    Normal,
    /// Overlay a pantalla completa.
    Full,
    /// Un menú chico.
    Compact,
}

impl MenuDisplayMode {
    /// El modo que nombra una clave de la configuración, si nombra alguno.
    pub fn from_key(value: &str) -> Option<Self> {
        match value {
            "normal" => Some(Self::Normal),
            "full" => Some(Self::Full),
            "compact" => Some(Self::Compact),
            _ => None,
        }
    }
}

/// Ancho y alto del menú compacto, en píxeles lógicos.
pub const COMPACT_SIZE: (f64, f64) = (680.0, 460.0);

/// El modo que declara una configuración ya leída.
///
/// Se comprueba el valor en lugar de afirmarlo: el archivo se edita a mano. Sin
/// la clave `menu.displayMode`, o con algo que no sea de los tres, es `normal`.
pub fn from_json(content: &str) -> MenuDisplayMode {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .as_ref()
        .and_then(|config| config.get("menu"))
        .and_then(|menu| menu.get("displayMode"))
        .and_then(serde_json::Value::as_str)
        .and_then(MenuDisplayMode::from_key)
        .unwrap_or_default()
}

/// El modo que hay puesto ahora mismo. Sin archivo o ilegible: `normal`.
pub fn read() -> MenuDisplayMode {
    let Some(path) = config_path() else {
        return MenuDisplayMode::default();
    };

    match std::fs::read_to_string(&path) {
        Ok(content) => from_json(&content),
        Err(_) => MenuDisplayMode::default(),
    }
}

/// Con qué tamaño y anclaje abrir el menú (vasak-desktop#210).
///
/// - `Normal`: el del [`AppletSpec`] (900×620), colgado del botón.
/// - `Compact`: un tamaño fijo chico, colgado del botón.
/// - `Full`: un overlay a pantalla completa —los cuatro bordes, sin márgenes—,
///   que no cuelga de ningún botón (ver `anchored_applet::place_fullscreen`).
pub fn applet_sizing(mode: MenuDisplayMode) -> AppletSizing {
    match mode {
        MenuDisplayMode::Normal => AppletSizing::Default,
        MenuDisplayMode::Compact => AppletSizing::Fixed(COMPACT_SIZE.0, COMPACT_SIZE.1),
        MenuDisplayMode::Full => AppletSizing::Fullscreen,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_la_clave_el_menu_abre_normal() {
        assert_eq!(from_json("{}"), MenuDisplayMode::Normal);
        assert_eq!(from_json("{\"menu\":{}}"), MenuDisplayMode::Normal);
        // Un valor que no es de los tres cae en normal, no rompe el menú.
        assert_eq!(
            from_json("{\"menu\":{\"displayMode\":\"gigante\"}}"),
            MenuDisplayMode::Normal
        );
        // Un archivo ilegible también.
        assert_eq!(from_json("no es json"), MenuDisplayMode::Normal);
    }

    #[test]
    fn lee_los_tres_modos() {
        assert_eq!(
            from_json("{\"menu\":{\"displayMode\":\"normal\"}}"),
            MenuDisplayMode::Normal
        );
        assert_eq!(
            from_json("{\"menu\":{\"displayMode\":\"full\"}}"),
            MenuDisplayMode::Full
        );
        assert_eq!(
            from_json("{\"menu\":{\"displayMode\":\"compact\"}}"),
            MenuDisplayMode::Compact
        );
    }

    #[test]
    fn el_tamano_y_anclaje_es_el_esperado() {
        // Normal usa el del AppletSpec colgado del botón; compact un tamaño fijo
        // chico; full el overlay a pantalla completa.
        assert_eq!(
            applet_sizing(MenuDisplayMode::Normal),
            AppletSizing::Default
        );
        assert_eq!(
            applet_sizing(MenuDisplayMode::Compact),
            AppletSizing::Fixed(COMPACT_SIZE.0, COMPACT_SIZE.1)
        );
        assert_eq!(
            applet_sizing(MenuDisplayMode::Full),
            AppletSizing::Fullscreen
        );
    }
}
