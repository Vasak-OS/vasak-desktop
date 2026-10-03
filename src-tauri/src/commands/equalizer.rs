//! Los comandos del ecualizador de sistema: leer el estado y mandar lo que se
//! tocó. Ver `applets/equalizer.rs` para el contrato.

use crate::applets::equalizer::{check_gain, proxy, read_state, EqualizerState};
use crate::logger::log_error;
use zbus::Connection;

async fn session() -> Result<Connection, String> {
    Connection::session().await.map_err(|e| e.to_string())
}

/// El estado entero; sin el servicio, el de «no está».
#[tauri::command]
pub async fn equalizer_state() -> Result<EqualizerState, String> {
    Ok(read_state(&session().await?).await)
}

/// Mueve una banda. La interfaz lo manda mientras se arrastra, hasta unas 30
/// veces por segundo; el servicio guarda solo, medio segundo después.
#[tauri::command]
pub async fn equalizer_set_gain(band: u32, gain: f64) -> Result<(), String> {
    let conn = session().await?;
    let state = read_state(&conn).await;
    if !state.service {
        return Err("el ecualizador no está en el bus".into());
    }
    check_gain(band, gain, state.frequencies.len(), state.range)?;
    proxy(&conn).await?.set_gain(band, gain).await.map_err(|e| {
        log_error(&format!("[equalizer] SetGain({band}, {gain}): {e}"));
        e.to_string()
    })
}

/// Elige un perfil de fábrica, o `custom` para volver a los valores propios.
#[tauri::command]
pub async fn equalizer_set_preset(preset: String) -> Result<(), String> {
    let conn = session().await?;
    proxy(&conn).await?.set_preset(&preset).await.map_err(|e| {
        log_error(&format!("[equalizer] SetPreset({preset}): {e}"));
        e.to_string()
    })
}

/// Prende o apaga el filtro.
#[tauri::command]
pub async fn equalizer_set_enabled(enabled: bool) -> Result<(), String> {
    let conn = session().await?;
    proxy(&conn)
        .await?
        .set_enabled(enabled)
        .await
        .map_err(|e| e.to_string())
}
