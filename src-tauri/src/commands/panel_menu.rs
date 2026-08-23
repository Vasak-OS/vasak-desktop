//! El menú del clic derecho del panel.

use tauri::AppHandle;

use crate::windows_apps::open_panel_menu_window;

/// `x` es la posición del clic en la pantalla, para que el menú aparezca ahí.
#[tauri::command]
pub async fn open_panel_menu(app: AppHandle, x: i32) -> Result<(), String> {
    open_panel_menu_window(app, x).await
}
