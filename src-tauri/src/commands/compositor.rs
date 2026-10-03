//! Los comandos del panel en píldoras (vasak-desktop#151): los espacios de
//! trabajo, la distribución de teclado y la región que recibe el puntero.

use serde::Deserialize;
use tauri::AppHandle;

use crate::applets::compositor::{
    current_keyboard_layout, current_workspaces, cycle_keyboard_layout, switch_to, KeyboardLayout,
    WorkspaceState,
};
use crate::windows_apps::panel::PANEL_LABEL;
use crate::windows_apps::shell_layer::set_layer_input_rects;

/// Los espacios de la pantalla con foco. `None` si Wayfire no dice.
#[tauri::command]
pub async fn get_workspaces() -> Result<Option<WorkspaceState>, String> {
    current_workspaces().await
}

/// Pasa al espacio número `index` (desde 0) de la pantalla con foco.
#[tauri::command]
pub async fn switch_workspace(index: u32) -> Result<(), String> {
    switch_to(index).await
}

/// La distribución de teclado de ahora.
#[tauri::command]
pub async fn get_keyboard_layout() -> Result<Option<KeyboardLayout>, String> {
    current_keyboard_layout().await
}

/// Pasa a la distribución siguiente y devuelve la que quedó.
#[tauri::command]
pub async fn next_keyboard_layout() -> Result<Option<KeyboardLayout>, String> {
    cycle_keyboard_layout().await
}

/// Un rectángulo de la página del panel, en píxeles CSS, que con la escala del
/// WebView en 1 son los lógicos de la superficie.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct InputRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Redondea hacia afuera: un borde de media píxel de una píldora también se
/// tiene que poder tocar.
pub fn to_pixels(rect: &InputRect) -> (i32, i32, i32, i32) {
    let left = rect.x.floor();
    let top = rect.y.floor();
    let right = (rect.x + rect.width).ceil();
    let bottom = (rect.y + rect.height).ceil();
    (
        left as i32,
        top as i32,
        (right - left).max(0.0) as i32,
        (bottom - top).max(0.0) as i32,
    )
}

/// Recorta lo que recibe el puntero en el panel a las píldoras.
///
/// La superficie del panel es la franja entera —la que reserva el lugar para
/// que las ventanas no queden debajo—, pero entre las píldoras se ve el
/// escritorio, y un clic ahí tiene que caer en el escritorio (su menú, sus
/// widgets) y no en una franja invisible. La página mide sus píldoras y manda
/// los rectángulos; una lista vacía deja la franja entera.
#[tauri::command]
pub fn set_panel_input_region(app: AppHandle, rects: Vec<InputRect>) -> Result<(), String> {
    let pixels: Vec<_> = rects.iter().map(to_pixels).collect();
    app.run_on_main_thread(move || {
        set_layer_input_rects(PANEL_LABEL, &pixels);
    })
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_bordes_con_coma_se_redondean_hacia_afuera() {
        let rect = InputRect {
            x: 10.4,
            y: 2.5,
            width: 31.2,
            height: 32.0,
        };
        // De 10 a 42 (41,6 hacia arriba) y de 2 a 35 (34,5 hacia arriba).
        assert_eq!(to_pixels(&rect), (10, 2, 32, 33));
    }

    #[test]
    fn un_rectangulo_entero_queda_igual() {
        let rect = InputRect {
            x: 4.0,
            y: 2.0,
            width: 120.0,
            height: 32.0,
        };
        assert_eq!(to_pixels(&rect), (4, 2, 120, 32));
    }

    #[test]
    fn un_ancho_negativo_no_da_un_rectangulo_al_reves() {
        let rect = InputRect {
            x: 4.0,
            y: 2.0,
            width: -10.0,
            height: 32.0,
        };
        assert_eq!(to_pixels(&rect).2, 0);
    }
}
