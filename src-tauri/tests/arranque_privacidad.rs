//! Cómo llega el indicador de cámara y micrófono a saber lo que muestra.
//!
//! Son tres decisiones que no fallan ninguna compilación cuando se rompen: se
//! ven como «a veces el icono no aparece», que es lo peor que puede pasarle a
//! un aviso de que te están grabando.

const LIB: &str = include_str!("../src/lib.rs");
const APPLET: &str = include_str!("../src/applets/privacidad.rs");
const COMPONENTE: &str = include_str!("../../src/components/buttons/TrayIconPrivacy.vue");
const COMANDOS: &str = include_str!("../src/commands/privacidad.rs");

/// La línea donde se registra un applet.
fn registro_de(applet: &str) -> &'static str {
    LIB.lines()
        .map(str::trim)
        .find(|linea| linea.starts_with(&format!("manager.register({applet},")))
        .unwrap_or_else(|| panic!("{applet} no está registrado en lib.rs"))
}

#[test]
fn el_applet_de_privacidad_es_diferido() {
    // Recorrer /proc no tiene por qué competir con lo que dibuja el panel, y
    // nadie abre la cámara en el primer segundo de sesión.
    assert!(
        registro_de("PrivacidadApplet").contains("AppletPriority::Deferred"),
        "el indicador no tiene que competir con el arranque del panel"
    );
}

#[test]
fn el_evento_que_emite_es_el_que_el_panel_escucha() {
    let emitido = APPLET
        .lines()
        .find_map(|linea| linea.trim().strip_prefix("const EVENTO: &str = \""))
        .and_then(|resto| resto.split('"').next())
        .expect("el applet ya no declara el nombre del evento");

    assert!(
        COMPONENTE.contains(&format!("'{emitido}'")),
        "el applet emite `{emitido}` y el panel escucha otra cosa"
    );
}

/// El panel se destruye y se vuelve a crear cuando cambian los monitores, y el
/// componente nuevo nace vacío. Como no se repite un anuncio igual al anterior,
/// sin esta consulta el icono se quedaría invisible con la cámara encendida.
#[test]
fn el_comando_para_preguntar_el_estado_esta_registrado() {
    assert!(
        LIB.contains("privacidad_en_uso,"),
        "el comando no está en el `invoke_handler`: la consulta del panel falla \
         en tiempo de ejecución y no lo dice ninguna compilación"
    );
    assert!(
        COMPONENTE.contains("privacyInUse"),
        "el componente no pregunta al montarse"
    );
}

/// La consulta sale antes de que el applet pueda anunciar y vuelve después.
#[test]
fn la_respuesta_vieja_no_pisa_a_la_nueva() {
    let dentro_del_montaje = COMPONENTE
        .split_once("onMounted(")
        .map(|(_, resto)| resto)
        .expect("el componente ya no consulta al montarse");

    assert!(
        dentro_del_montaje.contains("if (yaLlegoUnAnuncio.value) return;"),
        "aplicar la respuesta sin mirar pisa con la foto vieja lo que acaba de \
         llegar por el evento"
    );
}

/// Son dos vigilantes independientes sobre un mismo estado.
///
/// Mandar la foto entera —leyendo el campo del otro— le da a cada uno la
/// oportunidad de pisar con su copia vieja lo que el otro acaba de escribir, y
/// el indicador se queda mintiendo hasta el siguiente evento.
#[test]
fn cada_vigilante_toca_solo_su_campo() {
    let llamadas: Vec<&str> = APPLET
        .lines()
        .map(str::trim)
        .filter(|linea| linea.contains("anunciar(&app"))
        .collect();

    assert_eq!(
        llamadas.len(),
        7,
        "cambió la cantidad de anuncios: {llamadas:?}"
    );

    for llamada in llamadas {
        assert!(
            llamada.contains("|estado|") && !llamada.contains("EnUso {"),
            "este anuncio manda el estado entero en vez de su campo: {llamada}"
        );
    }
}

/// El botón de cortar del applet llama a un comando que tiene que existir del
/// otro lado. Tauri rechaza la llamada entera si no está, y eso se ve recién al
/// apretarlo — que es el peor momento, porque es cuando alguien quiere dejar de
/// compartir su pantalla.
#[test]
fn el_comando_para_cortar_esta_registrado() {
    assert!(
        COMANDOS.contains("pub async fn privacidad_cortar"),
        "el comando no existe"
    );
    assert!(
        LIB.contains("privacidad_cortar,"),
        "el comando no está en el `invoke_handler`"
    );
    assert!(
        LIB.contains("toggle_privacidad_applet,"),
        "el comando que abre el applet no está en el `invoke_handler`"
    );
}
