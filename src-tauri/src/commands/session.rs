use zbus::Connection;
use crate::logger::{log_info, log_error};

extern "C" {
    fn getuid() -> u32;
}

#[tauri::command]
pub fn detect_display_server() -> String {
    if std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland") {
        log_info("Servidor de display detectado: wayland");
        return "wayland".to_string();
    }

    if std::env::var("DISPLAY").is_ok() || std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("x11") {
        log_info("Servidor de display detectado: x11");
        return "x11".to_string();
    }

    log_info("Servidor de display no detectado, reportando: unknown");
    "unknown".to_string()
}

#[tauri::command]
pub async fn logout(_display_server: String) -> Result<(), String> {
    log_info("Cerrando sesión de usuario");
    // Usar D-Bus para terminar la sesión del usuario actual
    let connection = Connection::system()
        .await
        .map_err(|e| {
            log_error(&format!("No se pudo conectar a D-Bus para logout: {}", e));
            format!("No se pudo conectar a D-Bus: {}", e)
        })?;

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
        .map_err(|e| {
            log_error(&format!("Error al terminar sesión: {}", e));
            format!("Error al terminar sesión: {}", e)
        })?;

    log_info("Sesión cerrada correctamente");
    Ok(())
}

#[tauri::command]
pub async fn shutdown() -> Result<(), String> {
    log_info("Apagando el sistema");
    let connection = Connection::system()
        .await
        .map_err(|e| {
            log_error(&format!("No se pudo conectar a D-Bus para shutdown: {}", e));
            format!("No se pudo conectar a D-Bus: {}", e)
        })?;

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
    log_info("Reiniciando el sistema");
    let connection = Connection::system()
        .await
        .map_err(|e| {
            log_error(&format!("No se pudo conectar a D-Bus para reboot: {}", e));
            format!("No se pudo conectar a D-Bus: {}", e)
        })?;

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
    log_info("Suspendiendo el sistema");
    let connection = Connection::system()
        .await
        .map_err(|e| {
            log_error(&format!("No se pudo conectar a D-Bus para suspend: {}", e));
            format!("No se pudo conectar a D-Bus: {}", e)
        })?;

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