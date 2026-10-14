//! Los comandos de Tauri del calendario: un envoltorio fino sobre
//! [`crate::calendar_store`].
//!
//! Acá sólo vive lo que necesita el cliente del plugin y el `AppHandle` vivos:
//! la implementación de [`StoreCalls`] para [`StoreClient`] —cómo le habla al
//! sincronizador el escritorio de verdad— y los dos comandos que el tablero
//! invoca. Toda la lógica —juntar las páginas, el nombre del calendario, el
//! lugar, qué ve la persona ante cada error— y sus pruebas están en
//! `calendar_store`, que es un módulo contable para la cobertura; éste queda
//! excluido a propósito, porque es pegamento del bus que no se prueba en
//! unitarias.

use tauri::AppHandle;
use tauri_plugin_vsk_store::{StoreClient, StoreExt};

use crate::calendar_store::{
    classify_store, read_locations, read_occurrences, CalendarReply, EventRef, Failure, Request,
    StoreCalls, CALL_TIMEOUT, PAGE_LIMIT,
};
use crate::logger::log_info;

impl StoreCalls for StoreClient {
    async fn json(&self, request: Request<'_>) -> Result<String, Failure> {
        let method = request.method();
        let answer = match request {
            Request::Calendars => {
                tokio::time::timeout(CALL_TIMEOUT, self.call_json("ListCalendars", &())).await
            }
            Request::Occurrences { from, to, cursor } => {
                tokio::time::timeout(
                    CALL_TIMEOUT,
                    self.call_json(
                        "ListOccurrences",
                        &(from, to, Vec::<String>::new(), cursor, PAGE_LIMIT),
                    ),
                )
                .await
            }
            Request::Event {
                event_id,
                occurrence_id,
            } => {
                tokio::time::timeout(
                    CALL_TIMEOUT,
                    self.call_json("GetEvent", &(event_id, occurrence_id)),
                )
                .await
            }
        };
        answer
            .map_err(|_| {
                Failure::Failed(format!(
                    "{method} no contestó en {} s",
                    CALL_TIMEOUT.as_secs()
                ))
            })?
            .map_err(classify_store)
    }
}

/// Las veces de los eventos que caen en `[from, to)`, en RFC 3339.
#[tauri::command]
pub async fn calendar_occurrences(app: AppHandle, from: String, to: String) -> CalendarReply {
    let store = app.store();
    let reply = read_occurrences(store.as_ref(), &from, &to).await;
    if let CalendarReply::Failed { detail } = &reply {
        log_info(&format!(
            "[calendario] no se pudieron leer los eventos: {detail}"
        ));
    }
    reply
}

/// El lugar de unos eventos: los del día que muestra el tablero.
#[tauri::command]
pub async fn calendar_locations(app: AppHandle, events: Vec<EventRef>) -> Vec<String> {
    read_locations(app.store().as_ref(), &events).await
}
