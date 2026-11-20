//! Que el centro de control y el portapapeles tengan cada uno su propio
//! proceso de WebKit, y que ninguna otra superficie se aísle (vasak-desktop#309).
//!
//! `terminate_web_process()` baja el `WebKitWebProcess` entero, no una webview, y
//! WebKitGTK agrupa varias webviews por proceso mientras compartan `WebContext`.
//! Al esconder el centro y el portapapeles se llevaban puestos a los applets, el
//! menú, el popup de sesión y el menú del tray, que quedaban sin renderer. Aislar
//! esas dos superficies (un `data_directory` propio ⇒ `WebContext` propio ⇒
//! proceso propio) lo evita.
//!
//! Es glue de GTK/wry que no corre sin display: se usan guardas de
//! fuente, como en `webviews_residentes.rs`; el valor por omisión de
//! `LayerSpec` se prueba como unitaria en `shell_layer.rs`, porque `windows_apps`
//! no es público.

const SHELL_LAYER: &str = include_str!("../src/windows_apps/shell_layer.rs");
const CONTROL_CENTER: &str = include_str!("../src/windows_apps/control_center.rs");
const CLIPBOARD: &str = include_str!("../src/windows_apps/clipboard.rs");
const APPLET: &str = include_str!("../src/windows_apps/anchored_applet.rs");

/// Cuerpo de `spawn_layer_window`: hasta la llave de cierre a la columna cero.
fn spawn_body() -> &'static str {
    let start = SHELL_LAYER
        .find("pub fn spawn_layer_window")
        .expect("no se encontró `spawn_layer_window`");
    let rest = &SHELL_LAYER[start..];
    let end = rest
        .find("\n}\n")
        .expect("no se encontró el fin de la función");
    &rest[..end]
}

#[test]
fn el_data_directory_solo_se_aplica_bajo_isolated_process() {
    let body = spawn_body();
    let guard = body
        .find("if spec.isolated_process")
        .expect("`spawn_layer_window` debe condicionar el aislamiento a `spec.isolated_process`");
    let data_dir = body
        .find("data_directory(")
        .expect("`spawn_layer_window` debe fijar un `data_directory` para aislar el proceso");
    assert!(
        data_dir > guard,
        "`data_directory(` debe quedar detrás del `if spec.isolated_process`: \
         aplicarlo siempre aislaría todas las superficies (un proceso extra cada una)"
    );
    assert_eq!(
        body.matches("data_directory(").count(),
        1,
        "un único `data_directory`, el condicionado"
    );
}

#[test]
fn el_data_directory_aislado_es_por_superficie_y_absoluto() {
    // El método `data_directory` del builder guarda la ruta tal cual (no le une
    // la etiqueta, a diferencia de la que viene de tauri.conf.json). Por eso la
    // ruta tiene que armarse por superficie e ir absoluta: un literal fijo haría
    // caer al centro y al portapapeles en la misma clave del WebContextStore —
    // compartirían proceso entre sí y esconder uno mataría al otro—, y una ruta
    // relativa se crearía contra el cwd del proceso.
    let body = spawn_body();
    assert!(
        body.contains("app_local_data_dir"),
        "el `data_directory` aislado debe ser absoluto, bajo `app_local_data_dir`, no relativo al cwd"
    );
    assert!(
        body.contains(".join(label)"),
        "el `data_directory` aislado debe incluir la etiqueta de la ventana: una ruta fija haría \
         que el centro y el portapapeles compartan WebContext —y proceso— entre sí"
    );
}

#[test]
fn el_centro_de_control_y_el_portapapeles_piden_su_propio_proceso() {
    for (name, src) in [
        ("control_center.rs", CONTROL_CENTER),
        ("clipboard.rs", CLIPBOARD),
    ] {
        assert!(
            src.contains("isolated_process: true"),
            "{name} se mata al esconderse y debe tener su propio proceso de WebKit"
        );
    }
}

#[test]
fn los_applets_comparten_el_proceso_por_omision() {
    assert!(APPLET.contains("isolated_process: false"));
    assert!(
        !APPLET.contains("isolated_process: true"),
        "los applets no se aíslan: su proceso nunca se mata y cada uno costaría un proceso"
    );
}
