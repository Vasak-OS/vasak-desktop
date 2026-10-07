//! Los comandos del modo avión (vasak-desktop#180).

use crate::airplane_mode::AirplaneModeState;
use crate::logger::{log_error, log_info};

/// El estado del modo avión que ya se conoce. No toca `/dev/rfkill`.
#[tauri::command]
pub fn get_airplane_mode() -> AirplaneModeState {
    crate::airplane_mode::state()
}

/// Pone o quita el modo avión y devuelve el estado anterior.
#[tauri::command]
pub fn set_airplane_mode(enabled: bool) -> Result<bool, String> {
    log_info(&format!(
        "Modo avión: {}",
        if enabled { "poniendo" } else { "quitando" }
    ));
    crate::airplane_mode::set_enabled(enabled).map_err(|e| {
        log_error(&format!("No se pudo cambiar el modo avión: {e}"));
        e
    })
}

/// Desbloquea las radios de un tipo (`"wlan"` o `"bluetooth"`): lo usan los
/// mosaicos de Wi-Fi y Bluetooth al tocarlos con la radio bloqueada.
#[tauri::command]
pub fn unblock_radios(kind: String) -> Result<(), String> {
    let Some(code) = crate::airplane_mode::kind_from_name(&kind) else {
        return Err(format!("tipo de radio desconocido: {kind}"));
    };
    log_info(&format!("Modo avión: desbloqueando {kind}"));
    crate::airplane_mode::unblock(code).map_err(|e| {
        log_error(&format!("No se pudo desbloquear {kind}: {e}"));
        e
    })
}
