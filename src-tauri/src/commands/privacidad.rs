use crate::applets::privacidad::{EnUso, EstadoDePrivacidad};
use tauri::{AppHandle, Manager};

/// Qué está usando la cámara o el micrófono ahora mismo.
///
/// El panel no se crea una sola vez: cuando cambian los monitores se destruye y
/// se vuelve a levantar, y el componente nuevo nace vacío. Como `anunciar` no
/// repite lo que ya dijo, sin esta consulta el indicador quedaría invisible
/// hasta el próximo cambio de estado, con la cámara encendida y nadie
/// avisando.
///
/// Devuelve vacío mientras el applet no arrancó —es diferido— y en ese caso el
/// primer anuncio llega por el evento igual.
#[tauri::command]
pub async fn privacidad_en_uso(app: AppHandle) -> EnUso {
    let Some(estado) = app.try_state::<EstadoDePrivacidad>().map(|e| e.0.clone()) else {
        return EnUso::default();
    };

    let actual = estado.lock().await.clone();
    actual
}

/// Corta una captura de pantalla en curso.
///
/// El identificador es el de la sesión del portal, tal como llegó en la lista:
/// el agente lo reenvía a xdpw, que es lo único que la detiene de verdad.
/// Sacarla nada más de la lista dejaría a alguien creyendo que dejó de
/// compartir su pantalla sin que fuera cierto.
#[tauri::command]
pub async fn privacidad_cortar(sesion: String) -> Result<(), String> {
    crate::applets::privacidad::cortar_la_captura(&sesion).await
}

/// Abre o cierra el applet.
///
/// Esconder en vez de destruir, como los otros: el webview no se vuelve a
/// montar, así que `window-shown` es lo que hace que la vista vuelva a
/// preguntar qué hay.
#[tauri::command]
pub fn toggle_privacidad_applet(app: AppHandle) -> Result<(), String> {
    use tauri::{async_runtime::spawn, Emitter};

    if let Some(ventana) = app.get_webview_window("applet_privacidad") {
        if ventana.is_visible().unwrap_or(false) {
            let _ = ventana.hide();
        } else {
            let _ = ventana.emit("window-shown", ());
            let _ = ventana.show();
            let _ = ventana.set_focus();
        }
        return Ok(());
    }

    spawn(async move {
        if let Err(error) = crate::windows_apps::create_applet_privacidad_window(app).await {
            log::error!("[privacidad] no se pudo abrir el applet: {error}");
        }
    });

    Ok(())
}
