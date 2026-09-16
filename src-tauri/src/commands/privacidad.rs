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
