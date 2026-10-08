//! Si el panel se esconde solo.
//!
//! Sale de `panel.autohide` en `~/.config/vasak/vasak.conf`, la misma sección
//! que dice de qué lado va y qué aspecto tiene. `true` esconde el panel y lo
//! revela al pasar el cursor por el borde; cualquier otra cosa —o que no diga
//! nada— es `false`, que es como estuvo siempre: el panel fijo.
//!
//! Acá sólo importa para una cosa: **cuánto espacio reserva la superficie**. Con
//! auto-ocultar la zona exclusiva pasa a cero, así que las ventanas maximizadas
//! ocupan también la franja del panel; sin él, el panel reserva su franja como
//! siempre. El esconder y el revelar los hace la interfaz (`panel-autohide.ts`),
//! que recorta la región de entrada a una línea en el borde y desliza la barra.
//!
//! Se lee el archivo a mano, igual que la posición (`panel_position.rs`), y por
//! el mismo motivo: esto se necesita dentro del `setup` y del oyente de
//! `config-changed`, donde un `block_on` del gestor de configuración paniquea.

use crate::panel_position::config_path;

/// Si está declarado en una configuración ya leída.
///
/// Se comprueba el valor en lugar de afirmarlo: el archivo se edita a mano, y
/// ahí un `"si"` o un `1` no son `true`. La clave ausente —el caso normal, la
/// sección existe desde antes— vale por `false`: el panel no se esconde solo
/// hasta que alguien lo pide.
pub fn from_json(content: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .as_ref()
        .and_then(|config| config.get("panel"))
        .and_then(|panel| panel.get("autohide"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

/// Si el auto-ocultar está puesto ahora mismo.
///
/// Sin archivo, ilegible o sin la clave: `false`. Un panel que desaparece porque
/// la configuración no se pudo leer no es una opción.
pub fn read() -> bool {
    let Some(path) = config_path() else {
        return false;
    };

    match std::fs::read_to_string(&path) {
        Ok(content) => from_json(&content),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_la_clave_el_panel_no_se_esconde() {
        // La sección `panel` existe desde los indicadores y la posición; el caso
        // normal es que esté y esta clave no.
        assert!(!from_json("{}"));
        assert!(!from_json(r#"{"panel":{}}"#));
        assert!(!from_json(r#"{"panel":{"position":"top"}}"#));
    }

    #[test]
    fn solo_true_lo_prende() {
        assert!(from_json(r#"{"panel":{"autohide":true}}"#));
        assert!(!from_json(r#"{"panel":{"autohide":false}}"#));
    }

    #[test]
    fn cualquier_otra_cosa_vale_por_apagado() {
        // El archivo se edita a mano: un valor que no es booleano no puede dejar
        // la franja sin reservar por accidente.
        assert!(!from_json(r#"{"panel":{"autohide":"si"}}"#));
        assert!(!from_json(r#"{"panel":{"autohide":1}}"#));
        assert!(!from_json(r#"{"panel":"autohide"}"#));
        assert!(!from_json("no es json"));
        assert!(!from_json(""));
    }
}
