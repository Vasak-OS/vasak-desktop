//! De qué lado de la pantalla va el panel.
//!
//! Sale de `panel.position` en `~/.config/vasak/vasak.conf`, la misma
//! configuración que ya dice qué indicadores muestra el panel. Cuatro valores:
//! `top`, `bottom`, `left` y `right`; cualquier otra cosa —o que no diga nada—
//! vale por arriba, que es donde el panel estuvo siempre y donde la gente lo
//! busca.
//!
//! # Por qué se lee el archivo a mano y no por el plugin
//!
//! `read_config` del gestor de configuración es `async`, y esto se necesita en
//! dos lugares donde no se puede esperar: dentro del `setup` —que corre en el
//! hilo principal de GTK— y dentro del oyente de `config-changed`, que el
//! propio plugin emite **desde una tarea de Tokio**. Un `block_on` ahí adentro
//! paniquea con «cannot start a runtime from within a runtime». El archivo son
//! un par de kilobytes y esto se lee dos veces por cambio de configuración.
//!
//! Se respeta `VASAK_CONFIG_PATH` igual que el plugin: es lo que usan las
//! pruebas y las sesiones con la configuración en otro lado.

/// Lo que mide el panel en su lado corto, en píxeles lógicos.
///
/// El mismo de los cuatro lados: la barra no cambia de tamaño al moverse, sólo
/// de orientación. Lo necesitan también el centro de control y los applets, que
/// se apartan del panel a mano porque no reservan espacio.
pub const PANEL_THICKNESS: i32 = 38;

/// Lo que se apartan del borde del monitor las superficies que flotan sobre el
/// escritorio —el centro de control, los applets—, en píxeles lógicos.
pub const SCREEN_MARGIN: i32 = 10;

/// Dónde queda el panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelPosition {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

impl PanelPosition {
    /// Los cuatro lados, para recorrerlos en las pruebas.
    #[cfg(test)]
    pub const ALL: [PanelPosition; 4] = [Self::Top, Self::Bottom, Self::Left, Self::Right];

    /// La posición que nombra una clave de la configuración, si nombra alguna.
    pub fn from_key(value: &str) -> Option<Self> {
        match value {
            "top" => Some(Self::Top),
            "bottom" => Some(Self::Bottom),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            _ => None,
        }
    }

    /// Cómo se llama en el archivo: para el registro, y para decirle a la
    /// interfaz de qué lado crece un applet.
    pub fn key(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    /// A los costados el panel es una columna.
    pub fn is_vertical(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    /// Los bordes a los que se ancla la superficie: (izquierda, derecha, arriba, abajo).
    ///
    /// Siempre tres de los cuatro: los dos extremos del lado largo, para que la
    /// barra cruce la pantalla entera, y el borde contra el que se apoya. El
    /// cuarto queda suelto, y es el que le da el grosor.
    pub fn anchors(self) -> (bool, bool, bool, bool) {
        match self {
            Self::Top => (true, true, true, false),
            Self::Bottom => (true, true, false, true),
            Self::Left => (true, false, true, true),
            Self::Right => (false, true, true, true),
        }
    }

    /// Lo que mide la superficie en la pantalla que le toca, en píxeles lógicos.
    ///
    /// El grosor es el mismo en los cuatro lados; lo que cambia es sobre qué
    /// eje se estira.
    pub fn size(self, screen_width: f64, screen_height: f64) -> (f64, f64) {
        if self.is_vertical() {
            (PANEL_THICKNESS as f64, screen_height)
        } else {
            (screen_width, PANEL_THICKNESS as f64)
        }
    }

    /// Dónde empieza la superficie del panel dentro del monitor, en píxeles
    /// lógicos.
    ///
    /// Lo que la interfaz del panel mide con `getBoundingClientRect()` es
    /// relativo a la propia superficie; para ubicar algo en el monitor hay que
    /// sumarle esto. Arriba y a la izquierda el panel arranca en la esquina; abajo
    /// y a la derecha, a un grosor del borde de enfrente.
    pub fn origin(self, screen_width: f64, screen_height: f64) -> (f64, f64) {
        let thickness = PANEL_THICKNESS as f64;
        match self {
            Self::Top | Self::Left => (0.0, 0.0),
            Self::Bottom => (0.0, screen_height - thickness),
            Self::Right => (screen_width - thickness, 0.0),
        }
    }
}

/// La posición que declara una configuración ya leída.
///
/// Se comprueba el valor en lugar de afirmarlo: el archivo se edita a mano, y
/// ahí un `"izquierda"` no es ninguno de los cuatro lados. Es el mismo criterio
/// con el que lo lee el panel en la interfaz; leerlo distinto haría que la
/// superficie se ancle de un lado y lo de adentro se dibuje para el otro.
pub fn from_json(content: &str) -> PanelPosition {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .as_ref()
        .and_then(|config| config.get("panel"))
        .and_then(|panel| panel.get("position"))
        .and_then(serde_json::Value::as_str)
        .and_then(PanelPosition::from_key)
        .unwrap_or_default()
}

/// Dónde vive la configuración. Igual que en el gestor de configuración.
fn config_path() -> Option<std::path::PathBuf> {
    if let Some(set) = std::env::var_os("VASAK_CONFIG_PATH") {
        if !set.is_empty() {
            return Some(std::path::PathBuf::from(set));
        }
    }

    dirs::home_dir().map(|home| home.join(".config/vasak/vasak.conf"))
}

/// La posición que hay puesta ahora mismo.
///
/// Sin archivo, ilegible o sin la clave: arriba. Un panel que no aparece porque
/// la configuración no se pudo leer no es una opción.
pub fn read() -> PanelPosition {
    let Some(path) = config_path() else {
        return PanelPosition::default();
    };

    match std::fs::read_to_string(&path) {
        Ok(content) => from_json(&content),
        Err(_) => PanelPosition::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_nada_puesto_el_panel_va_arriba() {
        // Es donde estuvo siempre. La sección `panel` existe desde antes que
        // esta clave —lleva los interruptores de los indicadores—, así que el
        // caso normal es que la sección esté y la clave no.
        assert_eq!(from_json("{}"), PanelPosition::Top);
        assert_eq!(from_json(r#"{"panel":{}}"#), PanelPosition::Top);
        assert_eq!(
            from_json(r#"{"panel":{"weather":false}}"#),
            PanelPosition::Top
        );
    }

    #[test]
    fn los_cuatro_lados_se_leen() {
        let expected = [
            ("top", PanelPosition::Top),
            ("bottom", PanelPosition::Bottom),
            ("left", PanelPosition::Left),
            ("right", PanelPosition::Right),
        ];

        for (key, position) in expected {
            let content = format!(r#"{{"panel":{{"position":"{key}"}}}}"#);
            assert_eq!(from_json(&content), position);
            assert_eq!(position.key(), key);
        }
    }

    #[test]
    fn cualquier_otra_cosa_vale_por_arriba() {
        // El archivo se edita a mano, y un archivo a medio escribir tampoco
        // puede dejar la sesión sin panel.
        assert_eq!(
            from_json(r#"{"panel":{"position":"izquierda"}}"#),
            PanelPosition::Top
        );
        assert_eq!(from_json(r#"{"panel":{"position":3}}"#), PanelPosition::Top);
        assert_eq!(from_json(r#"{"panel":"left"}"#), PanelPosition::Top);
        assert_eq!(from_json("no es json"), PanelPosition::Top);
        assert_eq!(from_json(""), PanelPosition::Top);
    }

    #[test]
    fn el_panel_cruza_la_pantalla_de_lado_a_lado() {
        // Tres anclas de cuatro: los dos extremos del lado largo y el borde
        // contra el que se apoya. Con dos, la barra quedaría flotando en un
        // trozo de pantalla.
        for position in PanelPosition::ALL {
            let (left, right, top, bottom) = position.anchors();
            let count = [left, right, top, bottom]
                .iter()
                .filter(|set| **set)
                .count();
            assert_eq!(count, 3, "{:?}", position);
        }

        // Y el borde suelto es el de enfrente: el panel de arriba no toca abajo.
        assert_eq!(PanelPosition::Top.anchors(), (true, true, true, false));
        assert_eq!(PanelPosition::Bottom.anchors(), (true, true, false, true));
        assert_eq!(PanelPosition::Left.anchors(), (true, false, true, true));
        assert_eq!(PanelPosition::Right.anchors(), (false, true, true, true));
    }

    #[test]
    fn el_grosor_es_el_mismo_de_los_cuatro_lados() {
        // Lo pidió así: la barra no cambia de tamaño al moverse, se acomoda.
        let (width, height) = (1920.0, 1080.0);

        assert_eq!(
            PanelPosition::Top.size(width, height),
            (1920.0, PANEL_THICKNESS as f64)
        );
        assert_eq!(
            PanelPosition::Bottom.size(width, height),
            (1920.0, PANEL_THICKNESS as f64)
        );
        assert_eq!(
            PanelPosition::Left.size(width, height),
            (PANEL_THICKNESS as f64, 1080.0)
        );
        assert_eq!(
            PanelPosition::Right.size(width, height),
            (PANEL_THICKNESS as f64, 1080.0)
        );
    }

    #[test]
    fn a_los_costados_el_panel_es_una_columna() {
        assert!(!PanelPosition::Top.is_vertical());
        assert!(!PanelPosition::Bottom.is_vertical());
        assert!(PanelPosition::Left.is_vertical());
        assert!(PanelPosition::Right.is_vertical());
    }

    #[test]
    fn el_panel_de_abajo_y_el_de_la_derecha_arrancan_a_un_grosor_del_borde() {
        // Es lo que convierte lo que mide el panel —relativo a su superficie—
        // en un lugar del monitor. Con el panel abajo, un botón a 10 píxeles del
        // borde de la barra está a 1052 del borde de arriba de la pantalla.
        let (width, height) = (1920.0, 1080.0);
        let thickness = PANEL_THICKNESS as f64;

        assert_eq!(PanelPosition::Top.origin(width, height), (0.0, 0.0));
        assert_eq!(PanelPosition::Left.origin(width, height), (0.0, 0.0));
        assert_eq!(
            PanelPosition::Bottom.origin(width, height),
            (0.0, height - thickness)
        );
        assert_eq!(
            PanelPosition::Right.origin(width, height),
            (width - thickness, 0.0)
        );
    }
}
