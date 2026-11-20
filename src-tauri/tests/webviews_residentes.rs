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
const CLIPBOARD: &str = include_str!("../src/commands/clipboard.rs");
const CONNECT: &str = include_str!("../src/commands/connect.rs");
const CONTROL_CENTER: &str = include_str!("../src/commands/control_center.rs");
const CLIPBOARD_WINDOW: &str = include_str!("../src/windows_apps/clipboard.rs");
const CONTROL_CENTER_WINDOW: &str = include_str!("../src/windows_apps/control_center.rs");
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
fn el_centro_y_el_portapapeles_programan_matar_su_proceso_en_on_hide() {
    // vasak-desktop#307: escondidos dejaban su WebKitWebProcess vivo toda la
    // sesión. El cierre normal es por pérdida de foco (`dismiss_on_unfocus`), que
    // esconde la ventana sin pasar por el comando: por eso el armado va en el
    // `on_hide` de la superficie, que salta en toda forma de esconder. Los
    // comandos sólo cancelan al mostrar y no destruyen (vasak-desktop#264).
    for (window_source, create, command_source, toggle) in [
        (
            CONTROL_CENTER_WINDOW,
            "fn create_control_center_window",
            CONTROL_CENTER,
            "fn toggle_control_center",
        ),
        (
            CLIPBOARD_WINDOW,
            "fn create_clipboard_window",
            CLIPBOARD,
            "fn toggle_clipboard",
        ),
    ] {
        let create_body = body_of(window_source, create);
        assert!(
            create_body.contains("on_hide: {") && create_body.contains("arm_terminate_after_hide"),
            "`{create}` tiene que cablear `on_hide` con `arm_terminate_after_hide`: \
             sin eso el cierre por pérdida de foco no programa matar el proceso"
        );
        assert!(
            !destroys_a_surface(create_body),
            "`{create}` no debe destruir la superficie"
        );
        let toggle_body = body_of(command_source, toggle);
        assert!(
            toggle_body.contains("cancel_terminate"),
            "`{toggle}` tiene que cancelar el temporizador al mostrar"
        );
        assert!(
            !destroys_a_surface(toggle_body),
            "`{toggle}` no debe destruir en línea: va diferido"
        );
    }
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

/// Tras `if result.is_ok() {` del cuerpo, la llamada de arranque a
/// `arm_terminate_after_hide(app, <label>)`; `None` si falta la rama o la llamada.
fn boot_arm<'a>(body: &'a str, label: &str) -> Option<&'a str> {
    let after_ok = body.split("if result.is_ok() {").nth(1)?;
    let call = format!("arm_terminate_after_hide(app, {label})");
    after_ok.contains(&call).then_some(after_ok)
}

#[test]
fn el_centro_y_el_portapapeles_programan_matar_su_proceso_ya_al_crearse() {
    // vasak-desktop#307: la superficie nace escondida y `on_hide` no salta si
    // nadie la abre nunca; sin el armado de arranque su WebKitWebProcess (~240 MB)
    // vivía toda la sesión. Sólo se arma si la superficie se creó bien.
    for (source, create, label) in [
        (
            CONTROL_CENTER_WINDOW,
            "fn create_control_center_window",
            "CONTROL_CENTER_LABEL",
        ),
        (
            CLIPBOARD_WINDOW,
            "fn create_clipboard_window",
            "CLIPBOARD_LABEL",
        ),
    ] {
        let body = body_of(source, create);
        assert!(
            boot_arm(body, label).is_some(),
            "`{create}` tiene que programar la terminación al arranque: \
             `if result.is_ok() {{ arm_terminate_after_hide(app, {label}) }}`"
        );
    }
}
