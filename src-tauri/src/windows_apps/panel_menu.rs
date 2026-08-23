//! La ventana del menú del clic derecho del panel.
//!
//! El panel es una franja de unos treinta píxeles de alto pegada al borde de la
//! pantalla: un menú dibujado adentro quedaría recortado a la primera línea. Los
//! popups de la bandeja resuelven lo mismo con una ventana aparte, y este menú
//! usa el mismo camino.
//!
//! La ventana se crea cada vez y se cierra al perder el foco. Mantenerla viva y
//! oculta ahorraría unos milisegundos y costaría un webview permanente en
//! memoria por algo que se abre unas pocas veces al día.

use tauri::{AppHandle, Manager, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder};

use crate::{app_url::get_app_url, monitor_manager::get_primary_monitor};

pub const LABEL: &str = "panel_menu";

/// El tamaño lo fija Rust porque la ventana nace con un tamaño y no puede
/// crecer después sin que se vea el salto. Alcanza para los ítems que tiene.
const WIDTH: f64 = 260.0;
const HEIGHT: f64 = 168.0;

/// Cuánto se separa del borde de arriba: el panel más un poco de aire.
const OFFSET_Y: i32 = 44;

/// Cuánto se respeta del borde de la pantalla para que el menú no se corte.
const MARGIN: i32 = 8;

pub async fn open_panel_menu_window(app: AppHandle, x: i32) -> Result<(), String> {
    // Si ya estaba abierto, el clic derecho lo vuelve a poner donde se hizo, que
    // es lo que uno espera de un menú contextual.
    if let Some(previa) = app.get_webview_window(LABEL) {
        let _ = previa.close();
    }

    let monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;

    let window = WebviewWindowBuilder::new(
        &app,
        LABEL,
        WebviewUrl::App("index.html#/applets/panel-menu".into()),
    )
    .title("")
    .decorations(false)
    .transparent(true)
    .inner_size(WIDTH, HEIGHT)
    .max_inner_size(WIDTH, HEIGHT)
    .min_inner_size(WIDTH, HEIGHT)
    .visible(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .build()
    .map_err(|error| error.to_string())?;

    // La misma navegación explícita que hacen los otros popups: en desarrollo la
    // URL de la aplicación no es la del bundle.
    if let Ok(url) = Url::parse(&format!("{}/index.html#/applets/panel-menu", get_app_url())) {
        let _ = window.navigate(url);
    }

    window
        .set_position(Position::Physical(posicion(&monitor, x)))
        .map_err(|error| error.to_string())?;

    let _ = window.show();
    window.set_focus().map_err(|error| error.to_string())?;

    Ok(())
}

/// Justo debajo del panel, alineado al clic, sin salirse de la pantalla.
fn posicion(monitor: &tauri::Monitor, x: i32) -> PhysicalPosition<i32> {
    let tamano = monitor.size();
    let origen = monitor.position();

    PhysicalPosition {
        x: acomodar(x, origen.x, tamano.width as i32, WIDTH as i32),
        y: origen.y + OFFSET_Y,
    }
}

/// Corre el menú hacia adentro cuando el clic fue tan cerca del borde derecho
/// que el menú se saldría de la pantalla.
fn acomodar(deseado: i32, origen: i32, ancho_pantalla: i32, ancho_menu: i32) -> i32 {
    let maximo = origen + ancho_pantalla - ancho_menu - MARGIN;
    let minimo = origen + MARGIN;

    // El máximo primero y el mínimo después: en una pantalla más angosta que el
    // menú, el orden inverso lo dejaría fuera del borde izquierdo.
    (origen + deseado).min(maximo).max(minimo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en_el_medio_queda_donde_se_hizo_el_clic() {
        assert_eq!(acomodar(600, 0, 1920, 260), 600);
    }

    #[test]
    fn cerca_del_borde_derecho_se_corre_hacia_adentro() {
        // 1900 + 260 se saldría de una pantalla de 1920.
        assert_eq!(acomodar(1900, 0, 1920, 260), 1920 - 260 - MARGIN);
    }

    #[test]
    fn respeta_el_borde_izquierdo() {
        assert_eq!(acomodar(0, 0, 1920, 260), MARGIN);
    }

    #[test]
    fn en_un_monitor_secundario_suma_su_origen() {
        assert_eq!(acomodar(100, 1920, 1920, 260), 2020);
    }

    #[test]
    fn en_una_pantalla_mas_angosta_que_el_menu_no_se_va_por_la_izquierda() {
        assert_eq!(acomodar(150, 0, 200, 260), MARGIN);
    }
}
