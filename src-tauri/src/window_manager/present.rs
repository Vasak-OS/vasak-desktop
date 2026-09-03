//! Traer al frente la ventana de una aplicación.
//!
//! # Por qué esto existe
//!
//! En Wayland una aplicación **no puede** traerse sola al frente: es a
//! propósito, para que ninguna te robe el foco. El único que puede es el
//! compositor, y sólo lo hace por `xdg-activation` y con un token que se le pide
//! *mientras se tiene el foco*.
//!
//! Eso deja sin resolver tres casos que en un escritorio se dan todo el tiempo:
//!
//! - hacer clic en una notificación y que aparezca la aplicación que la mandó;
//! - abrir una sección de la configuración con la ventana ya abierta;
//! - cualquier lanzamiento desde el menú de algo que ya está corriendo.
//!
//! En los tres, quien pide no tiene token que pasar. El escritorio sí puede
//! hacerlo, porque habla con el compositor por su IPC. Así que lo hace acá, una
//! vez, y lo ofrece por D-Bus (`PresentApp`) para que no lo reimplemente cada
//! componente ni tenga que conocer a wayfire.
//!
//! Queda atado a wayfire, y es deliberado: la alternativa portable es que cada
//! llamador consiga y pase un token, que es más trabajo y **no cubre** el caso
//! de invocar desde una terminal, donde no hay foco del que sacarlo.

use super::wayfire_ipc::{get_wayfire_client, View};

/// Cuánto se parece un `app-id` a lo que se pidió. Cero es que no.
///
/// Los nombres llegan de lugares distintos y no coinciden nunca del todo: una
/// notificación dice `Telegram Desktop`, wayfire dice `org.telegram.desktop`, y
/// el menú puede decir `telegram-desktop`.
///
/// La comparación **no** es «uno contiene al otro», que es lo primero que uno
/// escribe y está mal: `desktop` está contenido en `org.telegram.desktop`, así
/// que pedir «escritorio» traería Telegram. Es la misma trampa que ya hizo que
/// Telegram apareciera en el panel con el icono de una carpeta, por recortar el
/// identificador hasta el último punto.
///
/// En su lugar se arman los nombres que ese identificador puede razonablemente
/// tener —el completo, el mismo sin el prefijo de dominio invertido, y el
/// segmento del programa— y se pide **igualdad**, o prefijo para los casos como
/// `firefox` contra `firefox-developer-edition`.
pub fn puntaje(app_id: &str, pedido: &str) -> u8 {
    let pedido = normalizar(pedido);
    if pedido.is_empty() || app_id.is_empty() {
        return 0;
    }

    let candidatos = nombres_posibles(app_id);
    if candidatos.contains(&pedido) {
        return 3;
    }
    // El mínimo de cuatro evita que un pedido cortito enganche por prefijo.
    if pedido.len() >= 4 && candidatos.iter().any(|c| c.starts_with(&pedido)) {
        return 2;
    }
    if candidatos
        .iter()
        .any(|c| c.len() >= 4 && pedido.starts_with(c.as_str()))
    {
        return 1;
    }
    0
}

/// Los nombres con los que un `app-id` se puede nombrar.
///
/// `org.telegram.desktop` da `orgtelegramdesktop`, `telegramdesktop` y
/// `telegram`: el completo, el que queda sin el dominio invertido —que es como
/// lo nombra la gente— y el del programa. `firefox` da sólo `firefox`.
/// Últimos segmentos que no nombran al programa, sólo dicen qué clase de cosa es.
const GENERICOS: &[&str] = &["desktop", "app", "client", "gui", "ui"];

fn nombres_posibles(app_id: &str) -> Vec<String> {
    let segmentos: Vec<&str> = app_id.split('.').collect();
    let mut nombres = vec![normalizar(app_id)];

    // Un dominio invertido tiene al menos tres segmentos: `org.telegram.desktop`.
    // Con dos —`vasak.settings`— no se puede distinguir el dominio del programa.
    if segmentos.len() >= 3 {
        nombres.push(normalizar(&segmentos[1..].join(".")));
        // El del fabricante: `org.telegram.desktop` es Telegram.
        nombres.push(normalizar(segmentos[1]));
        // Y el último, porque en la otra convención el programa va ahí:
        // `com.anthropic.Claude` es Claude. Salvo que sea una de las palabras
        // que no nombran a nadie — `org.telegram.desktop` no es «escritorio»—,
        // que es la trampa por la que Telegram salía en el panel con el icono
        // de una carpeta.
        if let Some(ultimo) = segmentos.last() {
            if !GENERICOS.contains(&ultimo.to_lowercase().as_str()) {
                nombres.push(normalizar(ultimo));
            }
        }
    }

    nombres.retain(|n| !n.is_empty());
    nombres
}

fn normalizar(valor: &str) -> String {
    valor
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Si esta vista es una ventana que se le pueda mostrar a alguien.
///
/// Deja afuera lo que no es una ventana de aplicación —paneles, fondos, menús—
/// y lo que el compositor no deja enfocar. Traer al frente algo de eso no haría
/// nada y, peor, ganaría la elección por sobre la ventana de verdad.
fn es_ventana(vista: &View) -> bool {
    vista.role.as_deref() == Some("toplevel")
        && vista.mapped.unwrap_or(true)
        && vista.focusable.unwrap_or(true)
}

/// Cuál de las ventanas hay que mostrar, si hay alguna.
///
/// Con varias del mismo programa gana la que se usó más recientemente, que es
/// la que la persona tiene en la cabeza cuando dice «mostrame Firefox».
pub fn elegir<'a>(vistas: &'a [View], pedido: &str) -> Option<&'a View> {
    vistas
        .iter()
        .filter(|v| es_ventana(v))
        .filter_map(|v| {
            let p = puntaje(v.app_id.as_deref().unwrap_or(""), pedido);
            (p > 0).then_some((p, v.last_focus_timestamp.unwrap_or(i64::MIN), v))
        })
        .max_by_key(|(p, t, _)| (*p, *t))
        .map(|(_, _, v)| v)
}

/// Trae al frente la ventana de una aplicación. Dice si encontró alguna.
///
/// Desminimizar va **antes** de enfocar y no al revés: enfocar una ventana
/// minimizada la deja enfocada y sin dibujar, que desde afuera se ve igual que
/// no haber hecho nada.
pub async fn present_app(pedido: &str) -> bool {
    let Some(cliente) = get_wayfire_client().await else {
        return false;
    };
    let Ok(vistas) = cliente.list_views_typed().await else {
        return false;
    };
    let Some(vista) = elegir(&vistas, pedido) else {
        return false;
    };

    if vista.minimized.unwrap_or(false) {
        let _ = cliente.set_minimized(vista.id as u64, false).await;
    }
    cliente.set_focus(vista.id as u64).await.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vista(id: i64, app_id: &str, ultimo_foco: i64) -> View {
        serde_json::from_value(serde_json::json!({
            "activated": false,
            "app-id": app_id,
            "id": id,
            "last-focus-timestamp": ultimo_foco,
            "mapped": true,
            "focusable": true,
            "role": "toplevel",
            "minimized": false,
        }))
        .expect("la vista de prueba")
    }

    /// Los nombres nunca coinciden del todo, y por eso se comparan así.
    #[test]
    fn el_mismo_programa_dicho_de_tres_maneras_es_el_mismo() {
        // Lo que dice una notificación, contra lo que dice wayfire.
        assert!(puntaje("org.telegram.desktop", "Telegram Desktop") > 0);
        assert!(puntaje("firefox", "Firefox") > 0);
        assert!(puntaje("vasak-settings", "vasak settings") > 0);
        assert!(puntaje("discord", "Discord") > 0);
    }

    /// La trampa que ya hizo aparecer a Telegram con el icono de una carpeta.
    ///
    /// Partir el `app-id` por puntos y quedarse con el último convierte
    /// `org.telegram.desktop` en `desktop`. Acá eso no puede pasar porque no se
    /// parte nada, pero la prueba fija que pedir «desktop» no traiga Telegram.
    #[test]
    fn pedir_desktop_no_trae_telegram() {
        let vistas = [vista(1, "org.telegram.desktop", 10)];
        assert!(elegir(&vistas, "desktop").is_none());
    }

    /// Con los identificadores reales de un escritorio andando.
    ///
    /// Los de arriba son casos armados; estos salieron de `list-views` sobre una
    /// sesión de verdad, y los nombres son los que esas aplicaciones ponen en
    /// `app_name` al mandar una notificación. Es la prueba que dice si esto
    /// sirve para lo que se hizo.
    #[test]
    fn los_identificadores_de_un_escritorio_de_verdad() {
        let vistas = [
            vista(1, "com.anthropic.Claude", 10),
            vista(2, "discord", 20),
            vista(3, "google-chrome", 30),
            vista(4, "org.telegram.desktop", 40),
            vista(5, "vasak-file-manager", 50),
            vista(6, "vasak-terminal", 60),
        ];

        for (pedido, esperado) in [
            ("Telegram Desktop", 4),
            ("Telegram", 4),
            ("Discord", 2),
            ("Google Chrome", 3),
            ("Claude", 1),
            ("vasak-terminal", 6),
            ("vasak-file-manager", 5),
        ] {
            assert_eq!(
                elegir(&vistas, pedido).map(|v| v.id),
                Some(esperado),
                "«{pedido}» tendría que traer la ventana {esperado}"
            );
        }

        // Y lo que no está abierto no trae cualquier otra cosa.
        assert!(elegir(&vistas, "Thunderbird").is_none());
    }

    #[test]
    fn un_pedido_cortito_no_engancha_cualquier_cosa() {
        // Sin el mínimo de cuatro, «mail» traería `thunderbird-mailnews` o lo
        // que sea que lo lleve adentro.
        assert_eq!(puntaje("org.gnome.Nautilus", "aut"), 0);
        assert_eq!(puntaje("firefox", ""), 0);
        assert_eq!(puntaje("", "firefox"), 0);
    }

    #[test]
    fn lo_que_no_tiene_nada_que_ver_no_coincide() {
        assert_eq!(puntaje("firefox", "discord"), 0);
        assert_eq!(puntaje("org.telegram.desktop", "vasak-settings"), 0);
    }

    #[test]
    fn el_exacto_le_gana_al_parecido() {
        // `firefox` y `firefox-developer-edition` son dos programas distintos.
        let vistas = [
            vista(1, "firefox-developer-edition", 99),
            vista(2, "firefox", 1),
        ];
        assert_eq!(elegir(&vistas, "firefox").map(|v| v.id), Some(2));
    }

    #[test]
    fn entre_dos_ventanas_del_mismo_programa_gana_la_ultima_usada() {
        let vistas = [vista(1, "firefox", 10), vista(2, "firefox", 50)];
        assert_eq!(elegir(&vistas, "firefox").map(|v| v.id), Some(2));
    }

    /// Un panel o un fondo no son ventanas que mostrarle a nadie.
    ///
    /// Y esto importa más de lo que parece: el escritorio propio tiene
    /// superficies con `app-id` que se parecen a lo que se pide, así que sin el
    /// filtro le ganarían la elección a la ventana de verdad y «mostrame la
    /// configuración» no haría nada visible.
    #[test]
    fn lo_que_no_es_una_ventana_queda_afuera() {
        let mut panel = vista(1, "vasak-desktop", 99);
        panel.role = Some("desktop-environment".into());
        let mut oculta = vista(2, "vasak-desktop", 98);
        oculta.mapped = Some(false);
        let mut sin_foco = vista(3, "vasak-desktop", 97);
        sin_foco.focusable = Some(false);
        let real = vista(4, "vasak-desktop", 1);

        let vistas = [panel, oculta, sin_foco, real];
        assert_eq!(elegir(&vistas, "vasak-desktop").map(|v| v.id), Some(4));
    }

    #[test]
    fn sin_ninguna_ventana_no_se_inventa_una() {
        assert!(elegir(&[], "firefox").is_none());
    }
}
