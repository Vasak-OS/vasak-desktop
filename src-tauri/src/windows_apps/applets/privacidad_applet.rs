//! La ventana del applet de privacidad.
//!
//! Lista quién está usando la cámara, el micrófono y la pantalla, y deja
//! cortar las capturas de pantalla, que es lo único de los tres que se puede
//! retirar en curso.
//!
//! Existe como ventana y no como tooltip por lo que hay adentro: el tooltip
//! puede nombrar, pero no puede tener un botón. Y el diálogo de captura viene
//! prometiendo desde siempre que se puede dejar de compartir «desde el
//! indicador de la barra».

use std::sync::Arc;

use tauri::{
    AppHandle, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

use crate::{app_url::get_app_url, monitor_manager::get_primary_monitor};

/// Alcanza para las tres listas con varias aplicaciones en cada una sin que
/// haya que desplazar en el caso normal, que es una o dos.
const WIDTH: f64 = 420.0;
const HEIGHT: f64 = 420.0;

pub async fn create_applet_privacidad_window(
    app: AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;

    let window = WebviewWindowBuilder::new(
        &app,
        "applet_privacidad",
        WebviewUrl::App("index.html#/applets/privacidad".into()),
    )
    .title("Vasak Privacy Applet")
    .decorations(false)
    .transparent(true)
    .inner_size(WIDTH, HEIGHT)
    .max_inner_size(WIDTH, HEIGHT)
    .min_inner_size(WIDTH, HEIGHT)
    .visible(true)
    .build()?;

    // Se cierra al perder el foco, como los otros applets.
    let al_perder_foco = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Focused(false)) {
            let _ = al_perder_foco.close();
        }
    });

    let ventana = Arc::new(window);

    if let Ok(url) = Url::parse(&format!("{}/index.html#/applets/privacidad", get_app_url())) {
        let _ = ventana.navigate(url);
    }

    let tamano = primary_monitor.size();
    let origen = primary_monitor.position();

    ventana.set_position(Position::Physical(PhysicalPosition {
        x: origen.x + (tamano.width as i32 / 2) - (WIDTH as i32 / 2),
        y: origen.y + (tamano.height as i32 / 2) - (HEIGHT as i32 / 2),
    }))?;

    ventana.set_focus()?;

    Ok(())
}
