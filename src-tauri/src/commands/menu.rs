use crate::menu_manager::get_menu_cached;
use crate::panel_position::PanelPosition;
use crate::structs::CategoryInfo;
use crate::windows_apps::anchored_applet::AnchorRect;
use crate::windows_apps::menu::{self, MenuButton};
use std::collections::HashMap;
use tauri::AppHandle;

/// Async so the scan — when the cache is cold — runs off the main thread; a
/// sync command would block the UI for the whole read of every .desktop file.
#[tauri::command]
pub async fn get_menu_items() -> HashMap<String, CategoryInfo> {
    get_menu_cached()
}

/// Abre o cierra el menú, colgado de su botón del panel.
///
/// `anchor` es el rectángulo del botón que se tocó, igual que en
/// `toggle_applet`. Sin él —D-Bus, la tecla Super— se ancla igual al botón del
/// menú: ver `windows_apps/menu.rs`.
///
/// Cerrar **esconde** la superficie y no la destruye, así que la página no se
/// recarga en cada apertura; la lista de aplicaciones se mantiene al día por
/// `menu_watcher`.
#[tauri::command]
pub fn toggle_menu(app: AppHandle, anchor: Option<AnchorRect>) -> Result<(), String> {
    menu::toggle_menu(&app, anchor)
}

/// El panel avisa dónde dibujó el botón del menú y de qué lado estaba.
///
/// Lo manda al montarse y cada vez que se reacomoda, para que abrir el menú sin
/// clic lo cuelgue del botón que se ve y no de uno supuesto.
#[tauri::command]
pub fn set_menu_button(side: String, anchor: AnchorRect) -> Result<(), String> {
    let side = PanelPosition::from_key(&side)
        .ok_or_else(|| format!("«{side}» no es un lado del panel"))?;
    menu::remember_menu_button(MenuButton { side, rect: anchor });
    Ok(())
}
