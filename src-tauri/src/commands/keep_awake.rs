//! «Mantener despierto» (vasak-desktop#179). La lógica está en
//! `crate::keep_awake`; acá sólo la entrada desde el frontend.

use tauri::AppHandle;

use crate::keep_awake::KeepAwakeState;

/// El estado del modo. Con el modo apagado pregunta una vez al bus del sistema
/// si logind está; se llama al montar el mosaico, no en un bucle.
#[tauri::command]
pub async fn get_keep_awake(app: AppHandle) -> KeepAwakeState {
    crate::keep_awake::state(&app).await
}

/// Lo pone o lo quita y devuelve si estaba puesto. Las ventanas se enteran por
/// el evento `keep-awake-changed`.
#[tauri::command]
pub async fn set_keep_awake(app: AppHandle, enabled: bool) -> Result<bool, String> {
    crate::keep_awake::set_enabled(&app, enabled).await
}
