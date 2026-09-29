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
pub async fn privacy_in_use(app: AppHandle) -> EnUso {
    let Some(state) = app.try_state::<EstadoDePrivacidad>().map(|e| e.0.clone()) else {
        return EnUso::default();
    };

    let current = state.lock().await.clone();
    current
}

/// Corta una captura de pantalla en curso.
///
/// El identificador es el de la sesión del portal, tal como llegó en la lista:
/// el agente lo reenvía a xdpw, que es lo único que la detiene de verdad.
/// Sacarla nada más de la lista dejaría a alguien creyendo que dejó de
/// compartir su pantalla sin que fuera cierto.
#[tauri::command]
pub async fn privacy_stop_screen(session: String) -> Result<(), String> {
    crate::applets::privacidad::cortar_la_captura(&session).await
}
