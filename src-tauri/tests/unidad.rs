//! Lo que la unidad de systemd y el código tienen que seguir diciendo igual.
//!
//! Son dos archivos que nadie ata: la unidad declara `Type=notify` y el aviso
//! vive en `src/listo.rs`. Si uno cambia sin el otro no falla ninguna
//! compilación ni ningún test — falla el arranque, y se ve sólo iniciando
//! sesión. Ya pasó una versión de esto: el aviso al socket abstracto fallaba en
//! silencio y la unidad se habría colgado hasta su tope en cada arranque.

use std::path::PathBuf;

fn unidad() -> String {
    let ruta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../packaging/systemd/user/vasak-desktop.service")
        .canonicalize()
        .expect("no se encontró la unidad empaquetada");
    std::fs::read_to_string(ruta).expect("no se pudo leer la unidad")
}

/// El valor de una directiva, ignorando comentarios.
///
/// Los comentarios de esta unidad **nombran las directivas que se descartaron**
/// y por qué, así que buscar por subcadena encontraría esas menciones antes que
/// la de verdad.
fn directiva(texto: &str, nombre: &str) -> Option<String> {
    texto
        .lines()
        .map(str::trim)
        .filter(|linea| !linea.starts_with('#'))
        .find_map(|linea| linea.strip_prefix(nombre)?.strip_prefix('=').map(str::trim))
        .map(str::to_string)
}

/// `Type=notify` es lo que hace que el inicio automático espere. Con
/// `Type=simple` systemd da la unidad por lista apenas hace `exec`, y volvemos a
/// tener a Steam dibujándose antes que el fondo.
#[test]
fn la_unidad_espera_el_aviso_del_escritorio() {
    assert_eq!(directiva(&unidad(), "Type").as_deref(), Some("notify"));
}

/// Y el aviso tiene que existir del lado del código.
#[test]
fn el_codigo_manda_ese_aviso() {
    let fuente =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/listo.rs"))
            .expect("no se pudo leer src/listo.rs");

    assert!(
        fuente.contains("READY=1"),
        "la unidad es Type=notify y el código no manda READY=1: systemd la mataría al vencerse TimeoutStartSec"
    );
}

/// Sin esto el inicio automático no queda detrás de nada y todo el cambio no
/// sirve para nada.
#[test]
fn el_inicio_automatico_queda_detras() {
    assert_eq!(
        directiva(&unidad(), "Before").as_deref(),
        Some("xdg-desktop-autostart.target")
    );
}

/// `[Install]` sólo hace efecto al habilitar la unidad, y a un paquete nadie le
/// corre `systemctl enable`: el enlace tiene que venir hecho.
#[test]
fn la_unidad_viene_habilitada_por_el_paquete() {
    let enlace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../packaging/systemd/user/graphical-session.target.wants/vasak-desktop.service");

    assert!(
        enlace.symlink_metadata().is_ok(),
        "falta el enlace en graphical-session.target.wants: la unidad no arrancaría nunca"
    );
    assert_eq!(
        directiva(&unidad(), "WantedBy").as_deref(),
        Some("graphical-session.target")
    );
}

/// El respaldo del código avisa a los 20 s; el tope de la unidad tiene que estar
/// por encima, o systemd mata al escritorio justo antes de que avise y el
/// respaldo no sirve para nada.
#[test]
fn el_tope_de_la_unidad_le_da_lugar_al_respaldo_del_codigo() {
    let tope: u64 = directiva(&unidad(), "TimeoutStartSec")
        .expect("la unidad tiene que declarar TimeoutStartSec")
        .parse()
        .expect("TimeoutStartSec en segundos, sin unidad");

    let fuente =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
            .expect("no se pudo leer src/lib.rs");
    assert!(
        fuente.contains("from_secs(20)"),
        "el respaldo dejó de ser de 20 s; hay que revisar TimeoutStartSec de la unidad"
    );

    assert!(
        tope > 20,
        "TimeoutStartSec={tope} no le deja lugar al respaldo de 20 s del propio escritorio"
    );
}

/// El volcado periódico tiene que arrancar **antes** de construir Tauri.
///
/// Todo lo que pasa después —los plugins, el setup— es justo el tramo que no
/// llegaba al archivo cuando el proceso moría, y es el que alguien quiere leer
/// cuando el escritorio no aparece. Arrancarlo después dejaría ese tramo sin
/// cubrir y no fallaría nada.
#[test]
fn el_volcado_periodico_arranca_antes_que_tauri() {
    let fuente =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
            .expect("no se pudo leer src/lib.rs");

    let volcado = fuente
        .find("logger::volcar_cada_tanto()")
        .expect("nadie arranca el volcado periódico del registro");
    let builder = fuente
        .find("tauri::Builder::default()")
        .expect("no se encontró la construcción de Tauri");

    assert!(
        volcado < builder,
        "el volcado periódico arranca después de construir Tauri: el tramo que se pierde \
         es justo el que hay que poder leer"
    );
}

/// Y la traza del arranque se vuelca a mano al terminar el setup, sin esperar
/// hasta cinco segundos. Es una escritura por sesión.
#[test]
fn el_setup_vuelca_su_traza_al_terminar() {
    let fuente =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
            .expect("no se pudo leer src/lib.rs");

    let fin_del_setup = fuente
        .find("Setup callback completed")
        .expect("no se encontró el final del setup");

    assert!(
        fuente[fin_del_setup..].contains("logger::flush()"),
        "el setup no vuelca su traza al terminar"
    );

    // Y el volcado tiene que ir **después** de la última línea del setup. Antes
    // deja fuera la que dice que el arranque terminó bien, que es la que
    // distingue «no llegó a configurarse» de «se configuró y murió después».
    let ultima = fuente
        .find("Aplicación Tauri configurada correctamente")
        .expect("no se encontró la última línea del setup");
    let volcado = fin_del_setup
        + fuente[fin_del_setup..]
            .find("logger::flush()")
            .expect("no se encontró el volcado del setup");

    assert!(
        volcado > ultima,
        "el volcado va antes de la última línea del setup, así que esa línea se queda en el búfer"
    );
}
