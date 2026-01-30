use crate::utils::shortcuts::shortcuts::{Shortcut, ShortcutsManager};
use crate::utils::shortcuts::shortcuts_handler::GlobalShortcutsHandler;
use crate::logger::{log_info, log_error, log_debug};
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
    log_debug("Obteniendo lista de shortcuts");
    let manager = state.manager.lock().map_err(|e| {
        log_error(&format!("Error bloqueando manager de shortcuts: {}", e));
        e.to_string()
    })?;
    let shortcuts = manager.get_shortcuts();
    log_debug(&format!("Obtenidos {} shortcuts", shortcuts.len()));
    Ok(shortcuts)
}

#[tauri::command]
pub fn update_shortcut(
    id: String,
    keys: String,
    state: State<ShortcutsState>,
) -> Result<Shortcut, String> {
    log_info(&format!("Actualizando shortcut '{}' a teclas: {}", id, keys));
    let mut manager = state.manager.lock().map_err(|e| e.to_string())?;
    let result = manager.update_shortcut(&id, &keys);
    if result.is_ok() {
        log_info(&format!("Shortcut '{}' actualizado correctamente", id));
    } else if let Err(ref e) = result {
        log_error(&format!("Error actualizando shortcut '{}': {}", id, e));
    }
    result
}

#[tauri::command]
pub fn add_custom_shortcut(
    name: String,
    description: String,
    keys: String,
    command: String,
    state: State<ShortcutsState>,
) -> Result<Shortcut, String> {
    log_info(&format!("Agregando shortcut personalizado: '{}' ({}) -> {}", name, keys, command));
    let mut manager = state.manager.lock().map_err(|e| e.to_string())?;
    let result = manager.add_custom_shortcut(name.clone(), description, keys, command);
    if result.is_ok() {
        log_info(&format!("Shortcut personalizado '{}' agregado correctamente", name));
    } else if let Err(ref e) = result {
        log_error(&format!("Error agregando shortcut personalizado '{}': {}", name, e));
    }
    result
}

#[tauri::command]
pub fn delete_shortcut(id: String, state: State<ShortcutsState>) -> Result<(), String> {
    log_info(&format!("Eliminando shortcut: {}", id));
    let mut manager = state.manager.lock().map_err(|e| e.to_string())?;
    let result = manager.delete_shortcut(&id);
    if result.is_ok() {
        log_info(&format!("Shortcut '{}' eliminado correctamente", id));
    } else if let Err(ref e) = result {
        log_error(&format!("Error eliminando shortcut '{}': {}", id, e));
    }
    result
}

#[tauri::command]
pub fn execute_shortcut(
    app: AppHandle,
    state: State<ShortcutsState>,
    shortcut_id: String,
) -> Result<(), String> {
    log_info(&format!("Ejecutando shortcut: {}", shortcut_id));
    let result = state.handler.execute_action(&shortcut_id, app);
    if result.is_ok() {
        log_info(&format!("Shortcut '{}' ejecutado correctamente", shortcut_id));
    } else if let Err(ref e) = result {
        log_error(&format!("Error ejecutando shortcut '{}': {}", shortcut_id, e));
    }
    result
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
    log_debug(&format!("Verificando conflictos para teclas: {}", keys));
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
        log_debug(&format!("Conflicto detectado: teclas '{}' ya en uso por '{}'", keys, dup.name));
        return Ok(ConflictInfo {
            has_conflict: true,
            conflict_with: Some(dup.name.clone()),
            message: format!("Este atajo ya está en uso por: {}", dup.name),
        });
    }

    log_debug(&format!("No hay conflictos para teclas: {}", keys));
    Ok(ConflictInfo {
        has_conflict: false,
        conflict_with: None,
        message: "No hay conflictos".to_string(),
    })
}
