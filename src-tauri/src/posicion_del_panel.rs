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
/// de orientación. Lo necesita también el centro de control, que se aparta del
/// panel a mano porque no reserva espacio.
pub const GROSOR_DEL_PANEL: i32 = 38;

/// Dónde queda el panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PosicionDelPanel {
    #[default]
    Arriba,
    Abajo,
    Izquierda,
    Derecha,
}

impl PosicionDelPanel {
    /// La posición que nombra una clave de la configuración, si nombra alguna.
    pub fn desde_clave(valor: &str) -> Option<Self> {
        match valor {
            "top" => Some(Self::Arriba),
            "bottom" => Some(Self::Abajo),
            "left" => Some(Self::Izquierda),
            "right" => Some(Self::Derecha),
            _ => None,
        }
    }

    /// Cómo se llama en el archivo, para el registro.
    pub fn clave(self) -> &'static str {
        match self {
            Self::Arriba => "top",
            Self::Abajo => "bottom",
            Self::Izquierda => "left",
            Self::Derecha => "right",
        }
    }

    /// A los costados el panel es una columna.
    pub fn vertical(self) -> bool {
        matches!(self, Self::Izquierda | Self::Derecha)
    }

    /// Los bordes a los que se ancla la superficie: (izquierda, derecha, arriba, abajo).
    ///
    /// Siempre tres de los cuatro: los dos extremos del lado largo, para que la
    /// barra cruce la pantalla entera, y el borde contra el que se apoya. El
    /// cuarto queda suelto, y es el que le da el grosor.
    pub fn anclas(self) -> (bool, bool, bool, bool) {
        match self {
            Self::Arriba => (true, true, true, false),
            Self::Abajo => (true, true, false, true),
            Self::Izquierda => (true, false, true, true),
            Self::Derecha => (false, true, true, true),
        }
    }

    /// Lo que mide la superficie en la pantalla que le toca, en píxeles lógicos.
    ///
    /// El grosor es el mismo en los cuatro lados; lo que cambia es sobre qué
    /// eje se estira.
    pub fn tamano(self, ancho_de_pantalla: f64, alto_de_pantalla: f64) -> (f64, f64) {
        if self.vertical() {
            (GROSOR_DEL_PANEL as f64, alto_de_pantalla)
        } else {
            (ancho_de_pantalla, GROSOR_DEL_PANEL as f64)
        }
    }
}

/// La posición que declara una configuración ya leída.
///
/// Se comprueba el valor en lugar de afirmarlo: el archivo se edita a mano, y
/// ahí un `"izquierda"` no es ninguno de los cuatro lados. Es el mismo criterio
/// con el que lo lee el panel en la interfaz; leerlo distinto haría que la
/// superficie se ancle de un lado y lo de adentro se dibuje para el otro.
pub fn desde_json(contenido: &str) -> PosicionDelPanel {
    serde_json::from_str::<serde_json::Value>(contenido)
        .ok()
        .as_ref()
        .and_then(|config| config.get("panel"))
        .and_then(|panel| panel.get("position"))
        .and_then(serde_json::Value::as_str)
        .and_then(PosicionDelPanel::desde_clave)
        .unwrap_or_default()
}

/// Dónde vive la configuración. Igual que en el gestor de configuración.
fn ruta_de_la_configuracion() -> Option<std::path::PathBuf> {
    if let Some(puesta) = std::env::var_os("VASAK_CONFIG_PATH") {
        if !puesta.is_empty() {
            return Some(std::path::PathBuf::from(puesta));
        }
    }

    dirs::home_dir().map(|home| home.join(".config/vasak/vasak.conf"))
}

/// La posición que hay puesta ahora mismo.
///
/// Sin archivo, ilegible o sin la clave: arriba. Un panel que no aparece porque
/// la configuración no se pudo leer no es una opción.
pub fn leer() -> PosicionDelPanel {
    let Some(ruta) = ruta_de_la_configuracion() else {
        return PosicionDelPanel::default();
    };

    match std::fs::read_to_string(&ruta) {
        Ok(contenido) => desde_json(&contenido),
        Err(_) => PosicionDelPanel::default(),
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
        assert_eq!(desde_json("{}"), PosicionDelPanel::Arriba);
        assert_eq!(desde_json(r#"{"panel":{}}"#), PosicionDelPanel::Arriba);
        assert_eq!(
            desde_json(r#"{"panel":{"weather":false}}"#),
            PosicionDelPanel::Arriba
        );
    }

    #[test]
    fn los_cuatro_lados_se_leen() {
        let esperados = [
            ("top", PosicionDelPanel::Arriba),
            ("bottom", PosicionDelPanel::Abajo),
            ("left", PosicionDelPanel::Izquierda),
            ("right", PosicionDelPanel::Derecha),
        ];

        for (clave, posicion) in esperados {
            let contenido = format!(r#"{{"panel":{{"position":"{clave}"}}}}"#);
            assert_eq!(desde_json(&contenido), posicion);
            assert_eq!(posicion.clave(), clave);
        }
    }

    #[test]
    fn cualquier_otra_cosa_vale_por_arriba() {
        // El archivo se edita a mano, y un archivo a medio escribir tampoco
        // puede dejar la sesión sin panel.
        assert_eq!(
            desde_json(r#"{"panel":{"position":"izquierda"}}"#),
            PosicionDelPanel::Arriba
        );
        assert_eq!(
            desde_json(r#"{"panel":{"position":3}}"#),
            PosicionDelPanel::Arriba
        );
        assert_eq!(desde_json(r#"{"panel":"left"}"#), PosicionDelPanel::Arriba);
        assert_eq!(desde_json("no es json"), PosicionDelPanel::Arriba);
        assert_eq!(desde_json(""), PosicionDelPanel::Arriba);
    }

    #[test]
    fn el_panel_cruza_la_pantalla_de_lado_a_lado() {
        // Tres anclas de cuatro: los dos extremos del lado largo y el borde
        // contra el que se apoya. Con dos, la barra quedaría flotando en un
        // trozo de pantalla.
        for posicion in [
            PosicionDelPanel::Arriba,
            PosicionDelPanel::Abajo,
            PosicionDelPanel::Izquierda,
            PosicionDelPanel::Derecha,
        ] {
            let (izq, der, arr, aba) = posicion.anclas();
            let cuantas = [izq, der, arr, aba]
                .iter()
                .filter(|puesta| **puesta)
                .count();
            assert_eq!(cuantas, 3, "{:?}", posicion);
        }

        // Y el borde suelto es el de enfrente: el panel de arriba no toca abajo.
        assert_eq!(PosicionDelPanel::Arriba.anclas(), (true, true, true, false));
        assert_eq!(PosicionDelPanel::Abajo.anclas(), (true, true, false, true));
        assert_eq!(
            PosicionDelPanel::Izquierda.anclas(),
            (true, false, true, true)
        );
        assert_eq!(
            PosicionDelPanel::Derecha.anclas(),
            (false, true, true, true)
        );
    }

    #[test]
    fn el_grosor_es_el_mismo_de_los_cuatro_lados() {
        // Lo pidió así: la barra no cambia de tamaño al moverse, se acomoda.
        let (ancho, alto) = (1920.0, 1080.0);

        assert_eq!(
            PosicionDelPanel::Arriba.tamano(ancho, alto),
            (1920.0, GROSOR_DEL_PANEL as f64)
        );
        assert_eq!(
            PosicionDelPanel::Abajo.tamano(ancho, alto),
            (1920.0, GROSOR_DEL_PANEL as f64)
        );
        assert_eq!(
            PosicionDelPanel::Izquierda.tamano(ancho, alto),
            (GROSOR_DEL_PANEL as f64, 1080.0)
        );
        assert_eq!(
            PosicionDelPanel::Derecha.tamano(ancho, alto),
            (GROSOR_DEL_PANEL as f64, 1080.0)
        );
    }

    #[test]
    fn a_los_costados_el_panel_es_una_columna() {
        assert!(!PosicionDelPanel::Arriba.vertical());
        assert!(!PosicionDelPanel::Abajo.vertical());
        assert!(PosicionDelPanel::Izquierda.vertical());
        assert!(PosicionDelPanel::Derecha.vertical());
    }
}
