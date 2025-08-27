use crate::menu_manager::get_menu;
use crate::structs::CategoryInfo;
use crate::windows_apps::create_menu_window;
use std::collections::HashMap;
use tauri::{async_runtime::spawn, AppHandle, Manager};

#[tauri::command]
pub fn get_menu_items() -> HashMap<String, CategoryInfo> {
    let menu_items = get_menu();
    menu_items
}

#[tauri::command]
pub fn toggle_menu(app: AppHandle) -> Result<(), ()> {
    if let Some(menu_window) = app.get_webview_window("menu") {
        if menu_window.is_visible().unwrap_or(false) {
            menu_window.close().expect("Failed to close menu window");
        } else {
            let _ = menu_window.show();
            let _ = menu_window.set_focus();
        }
    } else {
        spawn(async move {
            let _ = create_menu_window(app).await;
        });
    }

    Ok(())
}
