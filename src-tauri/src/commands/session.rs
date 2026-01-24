use std::env;
use zbus::Connection;

extern "C" {
    fn getuid() -> u32;
}

#[tauri::command]
pub fn detect_display_server() -> String {
    if env::var("WAYLAND_DISPLAY").is_ok() {
        "wayland".to_string()
    } else if env::var("DISPLAY").is_ok() {
        "x11".to_string()
    } else {
        "unknown".to_string()
    }
}

#[tauri::command]
pub async fn logout(_display_server: String) -> Result<(), String> {
    // Usar D-Bus para terminar la sesión del usuario actual
    let connection = Connection::system()
        .await
        .map_err(|e| format!("No se pudo conectar a D-Bus: {}", e))?;

    let uid = unsafe { getuid() };

    connection
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "TerminateUser",
            &(uid,),
        )
        .await
        .map_err(|e| format!("Error al terminar sesión: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn shutdown() -> Result<(), String> {
    let connection = Connection::system()
        .await
        .map_err(|e| format!("No se pudo conectar a D-Bus: {}", e))?;

    connection
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "PowerOff",
            &(true,),
        )
        .await
        .map_err(|e| format!("No se pudo ejecutar shutdown: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn reboot() -> Result<(), String> {
    let connection = Connection::system()
        .await
        .map_err(|e| format!("No se pudo conectar a D-Bus: {}", e))?;

    connection
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "Reboot",
            &(true,),
        )
        .await
        .map_err(|e| format!("No se pudo ejecutar reboot: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn suspend(_display_server: String) -> Result<(), String> {
    let connection = Connection::system()
        .await
        .map_err(|e| format!("No se pudo conectar a D-Bus: {}", e))?;

    connection
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "Suspend",
            &(true,),
        )
        .await
        .map_err(|e| format!("No se pudo ejecutar suspend: {}", e))?;

    Ok(())
}