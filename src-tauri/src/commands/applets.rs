use tauri::AppHandle;

use crate::windows_apps::anchored_applet::{
    dismiss_anchored_applet, open_applet_id, toggle_anchored_applet, AnchorRect, AppletSizing,
};

/// Abre o cierra un applet del panel, colgado del botón que lo pidió.
///
/// `applet` es uno de los de la tabla de `anchored_applet.rs`; `anchor`, el
/// rectángulo del botón según `getBoundingClientRect()`. Sin rectángulo —lo que
/// abre un applet desde fuera del panel— va centrado en el eje del panel.
#[tauri::command]
pub fn toggle_applet(
    app: AppHandle,
    applet: String,
    anchor: Option<AnchorRect>,
) -> Result<(), String> {
    // Sin tamaño propio: cada applet usa el de su `AppletSpec`. El menú pasa el
    // suyo por `toggle_menu`, según `menu.displayMode`.
    toggle_anchored_applet(&app, &applet, anchor, AppletSizing::Default)
}

/// Cierra con su animación el applet `applet`, si es el que está abierto.
///
/// La página se cierra con esto y no con el conmutador, por lo mismo que el
/// centro de control: esconder le saca el foco, perder el foco lo cierra, y el
/// conmutador lo encontraría cerrado y lo volvería a abrir.
#[tauri::command]
pub fn dismiss_applet(app: AppHandle, applet: String) -> Result<(), String> {
    dismiss_anchored_applet(&app, &applet)
}

/// Cierra lo que esté abierto —el applet o el menú, y el centro de control— por
/// un clic en el escritorio, que no les quita el foco (vasak-desktop#262).
#[tauri::command]
pub fn dismiss_popups(app: AppHandle) -> Result<(), String> {
    crate::commands::control_center::hide_control_center(app.clone()).ok();
    let handle = app.clone();
    // El estado de los applets vive en el hilo principal.
    app.run_on_main_thread(move || {
        if let Some(id) = open_applet_id() {
            let _ = dismiss_anchored_applet(&handle, id);
        }
    })
    .map_err(|error| format!("no se pudo llegar al hilo principal: {error}"))
}
