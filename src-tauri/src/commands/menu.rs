use std::collections::HashMap;
use crate::menu_manager::{get_menu, CategoryInfo};
use crate::windows_apps::create_menu_window;
use tauri::{AppHandle, Manager, async_runtime::spawn};

#[tauri::command]
pub fn get_menu_items() -> HashMap<String, CategoryInfo> {
    let menu_items = get_menu();
    menu_items
}

#[tauri::command]
pub fn toggle_menu(app: AppHandle) -> Result<(), ()> {
    let menu_window = app
        .get_webview_window("menu")
        .expect("menu window not found");

    if (menu_window.is_visible().unwrap()) {
        menu_window.close().expect("Failed to close menu window");
    }
    else {
        spawn(async move {
            create_menu_window(app).await.expect("Failed to create menu window");
        });
    }

    Ok(())
}