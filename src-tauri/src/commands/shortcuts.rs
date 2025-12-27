use crate::utils::shortcuts::shortcuts::{Shortcut, ShortcutsManager};
use crate::utils::shortcuts::shortcuts_handler::GlobalShortcutsHandler;
use tauri::{State, AppHandle};
use std::sync::Mutex;

pub struct ShortcutsState {
    pub manager: Mutex<ShortcutsManager>,
    pub handler: GlobalShortcutsHandler,
}

impl ShortcutsState {
    pub fn new() -> Self {
        Self {
            manager: Mutex::new(ShortcutsManager::new()),
            handler: GlobalShortcutsHandler::new(),
        }
    }
}

#[tauri::command]
pub fn get_shortcuts(state: State<ShortcutsState>) -> Result<Vec<Shortcut>, String> {
    let manager = state.manager.lock().map_err(|e| e.to_string())?;
    Ok(manager.get_shortcuts())
}

#[tauri::command]
pub fn update_shortcut(
    id: String,
    keys: String,
    state: State<ShortcutsState>,
) -> Result<Shortcut, String> {
    let mut manager = state.manager.lock().map_err(|e| e.to_string())?;
    manager.update_shortcut(&id, &keys)
}

#[tauri::command]
pub fn add_custom_shortcut(
    name: String,
    description: String,
    keys: String,
    command: String,
    state: State<ShortcutsState>,
) -> Result<Shortcut, String> {
    let mut manager = state.manager.lock().map_err(|e| e.to_string())?;
    manager.add_custom_shortcut(name, description, keys, command)
}

#[tauri::command]
pub fn delete_shortcut(id: String, state: State<ShortcutsState>) -> Result<(), String> {
    let mut manager = state.manager.lock().map_err(|e| e.to_string())?;
    manager.delete_shortcut(&id)
}

#[tauri::command]
pub fn execute_shortcut(
    app: AppHandle,
    state: State<ShortcutsState>,
    shortcut_id: String,
) -> Result<(), String> {
    state.handler.execute_action(&shortcut_id, app)
}

#[derive(serde::Serialize)]
pub struct ConflictInfo {
    pub has_conflict: bool,
    pub conflict_with: Option<String>,
    pub message: String,
}

#[tauri::command]
pub fn check_shortcut_conflicts(
    keys: String,
    exclude_id: Option<String>,
    state: State<ShortcutsState>,
) -> Result<ConflictInfo, String> {
    let manager = state.manager.lock().map_err(|e| e.to_string())?;
    let shortcuts = manager.get_shortcuts();

    // Normalizar las teclas para comparación
    let normalized_keys = keys.to_lowercase();

    // Buscar duplicados
    let duplicate = shortcuts.iter().find(|s| {
        let should_exclude = exclude_id.as_ref().map(|id| s.id == *id).unwrap_or(false);
        !should_exclude && s.keys.to_lowercase() == normalized_keys
    });

    if let Some(dup) = duplicate {
        return Ok(ConflictInfo {
            has_conflict: true,
            conflict_with: Some(dup.name.clone()),
            message: format!("Este atajo ya está en uso por: {}", dup.name),
        });
    }

    Ok(ConflictInfo {
        has_conflict: false,
        conflict_with: None,
        message: "No hay conflictos".to_string(),
    })
}
