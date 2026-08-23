//! La ventana del applet de Twingate.
//!
//! Va aparte del applet de red por lo que muestra: la lista de recursos con sus
//! vencimientos y los botones para autorizar es una pantalla en sí misma, y
//! metida adentro del applet de red empujaba hacia abajo lo que ese applet
//! tiene que responder primero —a qué red estoy conectado—.

use std::sync::Arc;

use tauri::{
    AppHandle, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

use crate::{app_url::get_app_url, monitor_manager::get_primary_monitor};

/// Alto y ancho pensados para la lista: setenta recursos no entran igual, pero
/// los que piden autorización sí, y de eso se trata la pantalla.
const WIDTH: f64 = 480.0;
const HEIGHT: f64 = 560.0;

pub async fn create_applet_twingate_window(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;

    let window = WebviewWindowBuilder::new(
        &app,
        "applet_twingate",
        WebviewUrl::App("index.html#/applets/twingate".into()),
    )
    .title("Vasak Twingate Applet")
    .decorations(false)
    .transparent(true)
    .inner_size(WIDTH, HEIGHT)
    .max_inner_size(WIDTH, HEIGHT)
    .min_inner_size(WIDTH, HEIGHT)
    .visible(true)
    .build()?;

    // Se cierra al perder el foco, como los otros applets: son ventanas que se
    // abren para mirar algo y se van solas.
    let al_perder_foco = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Focused(false)) {
            let _ = al_perder_foco.close();
        }
    });

    let ventana = Arc::new(window);

    if let Ok(url) = Url::parse(&format!("{}/index.html#/applets/twingate", get_app_url())) {
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
