use crate::menu_manager::get_menu_cached;
use crate::structs::CategoryInfo;
use crate::windows_apps::create_menu_window;
use crate::windows_apps::menu::MENU_LABEL;
use std::collections::HashMap;
use tauri::{async_runtime::spawn, AppHandle, Manager};

/// Async so the scan — when the cache is cold — runs off the main thread; a
/// sync command would block the UI for the whole read of every .desktop file.
#[tauri::command]
pub async fn get_menu_items() -> HashMap<String, CategoryInfo> {
    get_menu_cached()
}

#[tauri::command]
pub fn toggle_menu(app: AppHandle) -> Result<(), tauri::Error> {
    if let Some(menu_window) = app.get_webview_window(MENU_LABEL) {
        if menu_window.is_visible().unwrap_or(false) {
            // hide(), not close(): closing destroys the webview, so the next
            // open reloads the page and re-runs Vue from scratch. The app list
            // stays fresh through the menu_watcher instead.
            menu_window.hide()?;
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
