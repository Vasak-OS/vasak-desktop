use tauri::AppHandle;

use crate::audio::{
    get_volume, list_audio_devices, list_audio_input_devices, set_default_audio_device,
    set_default_audio_input_device, set_volume, toggle_mute,
};
use crate::logger::{log_debug, log_error, log_info};
use crate::structs::{AudioDevice, VolumeInfo};

// These are `async` so Tauri runs them on a worker instead of the main thread.
// Each one shells out to pactl through CommandExecutor, which spawns a thread
// and blocks the caller on it for up to three seconds — done on the main thread
// that stalls the panel, the clock and every animation while it waits.
#[tauri::command]
pub async fn get_audio_volume() -> Result<VolumeInfo, String> {
    log_debug("Comando: get_audio_volume");
    get_volume().map_err(|e| {
        log_error(&format!("Error al obtener volumen: {}", e));
        e.to_string()
    })
}

#[tauri::command]
pub async fn set_audio_volume(volume: i64, app: AppHandle) -> Result<(), String> {
    log_info(&format!("Estableciendo volumen a: {}%", volume));
    set_volume(volume, app).map_err(|e| {
        log_error(&format!("Error al establecer volumen: {}", e));
        e.to_string()
    })
}

#[tauri::command]
pub async fn toggle_audio_mute(app: AppHandle) -> Result<bool, String> {
    log_info("Alternando estado de mute");
    toggle_mute(app).map_err(|e| {
        log_error(&format!("Error al alternar mute: {}", e));
        e.to_string()
    })
}

#[tauri::command]
pub async fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    list_audio_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_audio_device(device_id: String, app: AppHandle) -> Result<bool, String> {
    log_info(&format!("Cambiando dispositivo de audio a: {}", device_id));
    set_default_audio_device(&device_id, app)
        .map(|_| {
            log_info("Dispositivo de audio cambiado correctamente");
            true
        })
        .map_err(|e| {
            log_error(&format!("Error al cambiar dispositivo de audio: {}", e));
            e.to_string()
        })
}

// ── El micrófono (vasak-desktop#182) ─────────────────────────────────────────
//
// La lectura sale de lo que el flujo de `pw-dump` ya trajo (`audio_input`), sin
// procesos; las órdenes van por `pactl` como las de la salida.

/// El volumen y el silencio de la fuente por omisión; `null` sin micrófono.
#[tauri::command]
pub async fn get_microphone() -> Result<Option<VolumeInfo>, String> {
    crate::audio::get_microphone().map_err(|e| {
        log_error(&format!("Error al leer el micrófono: {}", e));
        e.to_string()
    })
}

#[tauri::command]
pub async fn set_microphone_volume(volume: i64) -> Result<(), String> {
    crate::audio::set_microphone_volume(volume).map_err(|e| {
        log_error(&format!("Error al cambiar el volumen del micrófono: {}", e));
        e.to_string()
    })
}

/// Devuelve si el micrófono quedó silenciado.
#[tauri::command]
pub async fn toggle_microphone_mute() -> Result<bool, String> {
    crate::audio::toggle_microphone_mute().map_err(|e| {
        log_error(&format!("Error al silenciar el micrófono: {}", e));
        e.to_string()
    })
}

#[tauri::command]
pub async fn get_audio_input_devices() -> Result<Vec<AudioDevice>, String> {
    list_audio_input_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_audio_input_device(device_id: String, app: AppHandle) -> Result<bool, String> {
    set_default_audio_input_device(&device_id, app)
        .map(|_| true)
        .map_err(|e| {
            log_error(&format!("Error al cambiar la entrada de audio: {}", e));
            e.to_string()
        })
}
