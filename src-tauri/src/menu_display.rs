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
//! El tamaño de la superficie lo decide [`size_override`], que el conmutador del
//! menú le pasa al applet. `full` todavía **no** tiene su anclaje de overlay
//! propio (los cuatro bordes, sin márgenes, sin la animación desde el botón):
//! por ahora cae en un tamaño grande que el cálculo de ubicación acota al
//! monitor. El overlay de verdad lo agrega el backend (ver el TODO).

use crate::panel_position::config_path;

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

/// Un tamaño deliberadamente grande para `full`: el cálculo de ubicación lo
/// acota al monitor menos los márgenes, así que alcanza para un menú casi a
/// pantalla completa mientras no exista el anclaje de overlay propio.
const FULL_PLACEHOLDER: f64 = 10_000.0;

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

/// El tamaño con que abrir el menú, o `None` para el del [`AppletSpec`] (normal).
///
/// - `Normal`: `None` —el tamaño de siempre, 900×620—.
/// - `Compact`: el tamaño chico.
/// - `Full`: un tamaño grande que el cálculo de ubicación acota al monitor.
///
/// TODO(vasak-desktop#210): `full` de verdad es un overlay a pantalla completa
/// —anclaje layer-shell a los cuatro bordes, sin márgenes, tamaño = salida y sin
/// la animación que crece desde el botón—, que lo agrega el backend. Hasta
/// entonces cae en este tamaño grande acotado.
pub fn size_override(mode: MenuDisplayMode) -> Option<(f64, f64)> {
    match mode {
        MenuDisplayMode::Normal => None,
        MenuDisplayMode::Compact => Some(COMPACT_SIZE),
        MenuDisplayMode::Full => Some((FULL_PLACEHOLDER, FULL_PLACEHOLDER)),
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
    fn el_tamano_override_es_el_esperado() {
        // Normal usa el del AppletSpec (None); compact el chico; full un grande.
        assert_eq!(size_override(MenuDisplayMode::Normal), None);
        assert_eq!(size_override(MenuDisplayMode::Compact), Some(COMPACT_SIZE));
        let full = size_override(MenuDisplayMode::Full).expect("full tiene tamaño");
        assert!(full.0 >= 2000.0 && full.1 >= 2000.0);
    }
}
