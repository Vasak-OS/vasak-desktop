//! Que las superficies del escritorio se escondan y se reusen, nunca se
//! destruyan al cerrarse.
//!
//! La fuga era ésta: cada superficie de capa es un `WebKitWebView` re-parentado
//! en una ventana de GTK, y destruirlo al cerrarse deja vivo su
//! `WebKitWebProcess` —wry/GTK no lo reapea—, igual que en vasak-flare-daemon.
//! Una sesión real juntaba decenas de procesos colgados (~166 MB). El escritorio
//! ya cierra escondiendo (`hide_layer_window`) y reusa la superficie en la
//! próxima apertura; sólo un cambio de monitores las destruye, para rehacerlas en
//! la salida que corresponda. Esta guarda impide que el patrón que fugaba
//! —destruir al cerrar— vuelva al código.
//!
//! Es una guarda de fuente, no de comportamiento: la fuga es de ciclo de vida de
//! GTK/WebKit y de reaping de procesos, que no se puede montar sin un display ni
//! un WebKit reales. Se verifica además midiendo: con el arreglo el número de
//! WebKitWebProcess en reposo es estable y acotado (uno por superficie fija), no
//! crece con el uso (vasak-desktop#264).

const APPLET: &str = include_str!("../src/windows_apps/anchored_applet.rs");
const CONNECT: &str = include_str!("../src/commands/connect.rs");
const CONTROL_CENTER: &str = include_str!("../src/commands/control_center.rs");
const WEBKIT: &str = include_str!("../src/commands/webkit.rs");

/// El cuerpo de la primera función `fn <nombre>` del archivo, hasta su llave de
/// cierre a la columna cero. Dentro de una función todo está indentado, así que
/// `"\n}\n"` cae sólo en el cierre de la función y no en el de un bloque interno.
fn body_of<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source
        .find(signature)
        .unwrap_or_else(|| panic!("no se encontró `{signature}`"));
    let rest = &source[start..];
    let end = rest
        .find("\n}\n")
        .unwrap_or_else(|| panic!("no se encontró el fin de `{signature}`"));
    &rest[..end]
}

#[test]
fn cerrar_un_applet_lo_esconde_y_no_lo_destruye() {
    // Los tres caminos de cierre de un applet: el botón/Escape/foco
    // (`dismiss_on_main`), el cierre de golpe al abrir otro (`hide_now`) y el del
    // panel al mudarse de lado (`close_open_applet`). Ninguno destruye.
    for signature in ["fn dismiss_on_main", "fn hide_now", "fn close_open_applet"] {
        let body = body_of(APPLET, signature);
        assert!(
            body.contains("hide"),
            "`{signature}` tiene que cerrar escondiendo la superficie"
        );
        assert!(
            !destroys_a_surface(body),
            "`{signature}` no debe destruir el applet: destruir el webview \
             re-parentado deja vivo su WebKitWebProcess (fuga de vasak-desktop#264). \
             Se esconde y se reusa."
        );
    }
}

/// Si el cuerpo de una función destruye una superficie de capa. Mira las
/// llamadas reales (`.destroy(`, `destroy_layer_window`), no la palabra suelta:
/// algún comentario dice «never destroyed» justamente para explicar que no se
/// destruye.
fn destroys_a_surface(body: &str) -> bool {
    body.contains(".destroy(") || body.contains("destroy_layer_window")
}

#[test]
fn el_menu_del_telefono_se_esconde_al_cerrarse() {
    let body = body_of(CONNECT, "fn toggle_connect_menu");
    assert!(
        body.contains(".hide()"),
        "`toggle_connect_menu` tiene que esconder la ventana al cerrarla"
    );
    assert!(
        !body.contains(".close()") && !destroys_a_surface(body),
        "`toggle_connect_menu` no debe cerrar ni destruir la ventana: reusarla \
         evita la fuga del WebKitWebProcess y la recarga de la página."
    );
}

#[test]
fn el_centro_de_control_se_esconde_al_cerrarse() {
    let body = body_of(CONTROL_CENTER, "fn toggle_control_center");
    assert!(
        body.contains("hide_layer_window"),
        "`toggle_control_center` tiene que esconder la superficie al cerrarla"
    );
    assert!(
        !destroys_a_surface(body),
        "`toggle_control_center` no debe destruir la superficie: se esconde y se reusa."
    );
}

#[test]
fn el_reparenteo_afina_el_webkit_de_la_superficie() {
    // El afinado (vasak-desktop#265) va en el único camino por el que pasan todas
    // las superficies de capa. Si se saca, cada una vuelve a correr con los
    // valores por omisión de WebKitGTK.
    let body = body_of(WEBKIT, "fn reparent_webview");
    assert!(
        body.contains("tune_webkit"),
        "`reparent_webview` tiene que afinar el WebKit de la superficie al \
         re-parentearla, o cada superficie residente vuelve a los defaults."
    );
}
