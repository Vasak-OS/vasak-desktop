//! Cuándo arranca el indicador de cámara y micrófono.
//!
//! El componente del panel no pide el estado: se queda escuchando y espera el
//! primer evento. Eso funciona por una sola razón — el applet que lo publica es
//! **diferido**, y los diferidos arrancan después de que el panel avisó que
//! pintó, o sea después de que el componente se montó y se suscribió.
//!
//! Si alguien lo sube a `Normal` o a `Critical` buscando que aparezca antes, el
//! primer anuncio se emite contra nadie y el icono queda invisible hasta que
//! cambie algo. No falla ninguna compilación; se ve como «a veces no aparece».

const LIB: &str = include_str!("../src/lib.rs");

/// La línea donde se registra un applet.
fn registro_de(applet: &str) -> &'static str {
    LIB.lines()
        .map(str::trim)
        .find(|linea| linea.starts_with(&format!("manager.register({applet},")))
        .unwrap_or_else(|| panic!("{applet} no está registrado en lib.rs"))
}

#[test]
fn el_applet_de_privacidad_es_diferido() {
    assert!(
        registro_de("PrivacidadApplet").contains("AppletPriority::Deferred"),
        "el indicador tiene que arrancar después de que el panel pintó, o su \
         primer anuncio no lo escucha nadie"
    );
}

#[test]
fn el_evento_que_emite_es_el_que_el_panel_escucha() {
    let applet = include_str!("../src/applets/privacidad.rs");
    let componente = include_str!("../../src/components/buttons/TrayIconPrivacy.vue");

    let emitido = applet
        .lines()
        .find_map(|linea| linea.trim().strip_prefix("const EVENTO: &str = \""))
        .and_then(|resto| resto.split('"').next())
        .expect("el applet ya no declara el nombre del evento");

    assert!(
        componente.contains(&format!("'{emitido}'")),
        "el applet emite `{emitido}` y el panel escucha otra cosa"
    );
}
