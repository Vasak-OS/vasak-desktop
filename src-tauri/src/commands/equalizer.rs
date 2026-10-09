//! Los comandos del ecualizador de sistema: leer el estado y mandar lo que se
//! tocó. Ver `applets/equalizer.rs` para el contrato.
//!
//! Van por la conexión de sesión compartida (`DbusPool`): mientras se arrastra
//! una banda, `equalizer_set_gain` llega hasta unas 30 veces por segundo, y
//! abrir una conexión por llamada —autenticar, `Hello`— era la mayor parte del
//! trabajo. Por lo mismo, mover una banda no vuelve a leer el estado: se
//! comprueba contra el último leído, y si todavía no hay ninguno, decide el
//! servicio (`InvalidArgs`).

use crate::applets::equalizer::{
    check_gain, last_state, proxy, read_state, shared_session, EqualizerState,
};
use crate::logger::log_error;
use tauri::AppHandle;
use zbus::Connection;

async fn session(app: &AppHandle) -> Result<Connection, String> {
    shared_session(app).await.map_err(|e| e.to_string())
}

/// El estado entero; sin el servicio, el de «no está».
#[tauri::command]
pub async fn equalizer_state(app: AppHandle) -> Result<EqualizerState, String> {
    Ok(read_state(&session(&app).await?).await)
}

/// Mueve una banda. La interfaz lo manda mientras se arrastra, hasta unas 30
/// veces por segundo; el servicio guarda solo, medio segundo después.
#[tauri::command]
pub async fn equalizer_set_gain(app: AppHandle, band: u32, gain: f64) -> Result<(), String> {
    if let Some(state) = last_state().filter(|state| state.service) {
        check_gain(band, gain, state.frequencies.len(), state.range)?;
    } else if !gain.is_finite() {
        return Err(format!("la ganancia {gain} no es un número"));
    }
    let conn = session(&app).await?;
    proxy(&conn).await?.set_gain(band, gain).await.map_err(|e| {
        log_error(&format!("[equalizer] SetGain({band}, {gain}): {e}"));
        e.to_string()
    })
}

/// Elige un perfil de fábrica, o `custom` para volver a los valores propios.
#[tauri::command]
pub async fn equalizer_set_preset(app: AppHandle, preset: String) -> Result<(), String> {
    let conn = session(&app).await?;
    proxy(&conn).await?.set_preset(&preset).await.map_err(|e| {
        log_error(&format!("[equalizer] SetPreset({preset}): {e}"));
        e.to_string()
    })
}

/// Prende o apaga el filtro.
#[tauri::command]
pub async fn equalizer_set_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    let conn = session(&app).await?;
    proxy(&conn)
        .await?
        .set_enabled(enabled)
        .await
        .map_err(|e| e.to_string())
}
