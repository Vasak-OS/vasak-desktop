use tauri::AppHandle;

use crate::audio::{get_volume, set_volume, toggle_mute, list_audio_devices, set_default_audio_device};
use crate::structs::{VolumeInfo, AudioDevice};
use crate::windows_apps::applets::create_applet_audio_window;
use tauri::{async_runtime::spawn, Manager};

#[tauri::command]
pub fn get_audio_volume() -> Result<VolumeInfo, String> {
    get_volume().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_audio_volume(volume: i64, app: AppHandle) -> Result<(), String> {
    set_volume(volume, app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_audio_mute(app: AppHandle) -> Result<bool, String> {
    toggle_mute(app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    list_audio_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_audio_device(device_id: String, app: AppHandle) -> Result<bool, String> {
    set_default_audio_device(&device_id, app)
        .map(|_| true)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_audio_applet(app: AppHandle) -> Result<(), ()> {
    if let Some(audio_window) = app.get_webview_window("applet_audio") {
        if audio_window.is_visible().unwrap_or(false) {
            audio_window.close().expect("Failed to close audio window");
        } else {
            let _ = audio_window.show();
            let _ = audio_window.set_focus();
        }
    } else {
        spawn(async move {
            let _ = create_applet_audio_window(app).await;
        });
    }

    Ok(())
}
