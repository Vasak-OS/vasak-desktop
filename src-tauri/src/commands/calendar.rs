//! Los eventos del calendario, leídos del almacén local de vasak-accounts.
//!
//! El tablero de fecha (vasak-desktop#130) no habla con ningún servidor: le
//! pide los eventos al servicio que mantiene la copia local
//! (`vasak-accounts-sync`, decisión 6 del taller), que **responde siempre él**
//! —desde lo guardado, sin esperar a la red—. Es el mismo camino que van a usar
//! los widgets de calendario (#112) y `vasak-calendar`; no se copia ningún
//! cliente de CalDAV acá.
//!
//! La interfaz es `ar.net.vasak.os.AccountsStore`, en el nombre de bus del
//! sincronizador (`ar.net.vasak.os.AccountsSync`), en el bus de **sesión**.
//! Contesta JSON. Los métodos que se usan:
//!
//! - `ListOccurrences(from, to, calendar_ids, cursor, limit)`: las veces de los
//!   eventos de todas las cuentas que caen en `[from, to)`, por páginas.
//! - `ListCalendars()`: el nombre de cada calendario, para decir de cuál viene
//!   cada evento.
//! - `GetEvent(event_id, occurrence_id)`: el evento entero; de acá sale sólo el
//!   lugar, que `ListOccurrences` no trae.
//!
//! # El permiso, y por qué la llamada espera tanto
//!
//! Leer pide el permiso `store.calendar` de **quien llama**, y la primera vez
//! `vasak-permissions` abre un diálogo y contesta recién cuando la persona lo
//! cierra. El servicio espera hasta 120 s esa respuesta; acá se espera un poco
//! más ([`CALL_TIMEOUT`]), o la llamada vencería antes que la pregunta.
//!
//! # Lo que ve la persona según lo que pase
//!
//! Decisión 2 del taller: lo que no está **se ve no disponible, nunca roto**.
//! [`CalendarReply`] tiene las cuatro respuestas que el tablero sabe dibujar:
//! los eventos, «el servicio no está», «no diste permiso» y «falló», y cada
//! error de D-Bus cae en una de ellas ([`classify`]).

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use zbus::Connection;

use crate::dbus_pool::DbusPool;
use crate::logger::log_info;

const BUS_NAME: &str = "ar.net.vasak.os.AccountsSync";
const PATH: &str = "/ar/net/vasak/os/AccountsStore";
const INTERFACE: &str = "ar.net.vasak.os.AccountsStore";

/// Lo que se espera a una lectura: más que los 120 s que el servicio le da al
/// diálogo de permiso (`CHECK_TIMEOUT` de vasak-accounts).
const CALL_TIMEOUT: Duration = Duration::from_secs(150);

/// Cuántas veces de eventos se piden por página. El servicio no pasa de 1000.
const PAGE_LIMIT: u32 = 500;

/// Cuántas páginas se leen como mucho para un rango. Seis semanas con más de
/// cinco mil eventos no se dibujan igual; el resto queda marcado como recortado.
const MAX_PAGES: usize = 10;

/// Cuántos eventos se completan con su lugar de una vez: los de un día, que es
/// lo que muestra el tablero. Cada uno es una llamada.
const MAX_DETAILS: usize = 24;

/// Una vez de un evento, como la dibuja el tablero.
///
/// Los campos son los de `ListOccurrences`, más el nombre del calendario —de
/// `ListCalendars`— y el lugar —de `GetEvent`, sólo cuando se pide—.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalendarOccurrence {
    pub event_id: String,
    #[serde(default)]
    pub occurrence_id: String,
    #[serde(default)]
    pub calendar_id: String,
    pub title: String,
    /// RFC 3339 en UTC. Un día completo, a medianoche UTC.
    pub start: String,
    #[serde(default)]
    pub end: String,
    #[serde(default)]
    pub all_day: bool,
    /// La hora es la de quien mira: viaja en UTC pero se lee como local.
    #[serde(default)]
    pub floating: bool,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub calendar: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OccurrencePage {
    #[serde(default)]
    items: Vec<CalendarOccurrence>,
    #[serde(default)]
    next_cursor: Option<String>,
    #[serde(default)]
    truncated: bool,
}

#[derive(Debug, Deserialize)]
struct CalendarItem {
    id: String,
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct EventDetail {
    #[serde(default)]
    location: String,
}

/// Lo que contesta el tablero, en una de cuatro formas.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CalendarReply {
    /// Los eventos del rango. `truncated` si el servicio o el tope de páginas
    /// dejaron algo afuera.
    Ready {
        entries: Vec<CalendarOccurrence>,
        truncated: bool,
    },
    /// No hay servicio que conteste: no está instalado, no arrancó, o es una
    /// versión sin el almacén.
    Unavailable,
    /// La persona no le dio al escritorio el permiso de leer el calendario.
    Denied,
    /// Contestó con un error que no es ninguno de los dos de arriba.
    Failed { detail: String },
}

/// Cómo se lee un error de D-Bus.
#[derive(Debug, Clone, PartialEq)]
enum Failure {
    Unavailable,
    Denied,
    Failed(String),
}

impl From<Failure> for CalendarReply {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Unavailable => CalendarReply::Unavailable,
            Failure::Denied => CalendarReply::Denied,
            Failure::Failed(detail) => CalendarReply::Failed { detail },
        }
    }
}

/// Qué quiere decir el nombre de un error de D-Bus para quien mira el tablero.
///
/// Que el servicio no esté —ni el nombre en el bus, ni el objeto, ni el método,
/// que es lo que contesta una versión anterior al almacén— es «no disponible».
/// `AccessDenied` es la respuesta normal a un permiso negado.
fn classify_name(name: &str) -> Option<Failure> {
    match name {
        "org.freedesktop.DBus.Error.ServiceUnknown"
        | "org.freedesktop.DBus.Error.NameHasNoOwner"
        | "org.freedesktop.DBus.Error.UnknownObject"
        | "org.freedesktop.DBus.Error.UnknownInterface"
        | "org.freedesktop.DBus.Error.UnknownMethod"
        | "org.freedesktop.DBus.Error.Spawn.ServiceNotFound"
        | "org.freedesktop.DBus.Error.Spawn.ChildExited" => Some(Failure::Unavailable),
        "org.freedesktop.DBus.Error.AccessDenied" | "org.freedesktop.DBus.Error.AuthFailed" => {
            Some(Failure::Denied)
        }
        _ => None,
    }
}

fn classify(error: zbus::Error) -> Failure {
    match &error {
        zbus::Error::MethodError(name, detail, _) => classify_name(name.as_str())
            .unwrap_or_else(|| Failure::Failed(detail.clone().unwrap_or_else(|| name.to_string()))),
        zbus::Error::FDO(fdo) => match fdo.as_ref() {
            zbus::fdo::Error::ServiceUnknown(_)
            | zbus::fdo::Error::NameHasNoOwner(_)
            | zbus::fdo::Error::UnknownObject(_)
            | zbus::fdo::Error::UnknownInterface(_)
            | zbus::fdo::Error::UnknownMethod(_) => Failure::Unavailable,
            zbus::fdo::Error::AccessDenied(_) | zbus::fdo::Error::AuthFailed(_) => Failure::Denied,
            other => Failure::Failed(other.to_string()),
        },
        _ => Failure::Failed(error.to_string()),
    }
}

/// Una llamada al almacén que contesta JSON, con el tope de espera.
async fn call_json<A>(connection: &Connection, method: &str, args: &A) -> Result<String, Failure>
where
    A: serde::ser::Serialize + zbus::zvariant::DynamicType,
{
    let call = connection.call_method(Some(BUS_NAME), PATH, Some(INTERFACE), method, args);
    let reply = tokio::time::timeout(CALL_TIMEOUT, call)
        .await
        .map_err(|_| {
            Failure::Failed(format!(
                "{method} no contestó en {} s",
                CALL_TIMEOUT.as_secs()
            ))
        })?
        .map_err(classify)?;
    reply
        .body()
        .deserialize::<String>()
        .map_err(|e| Failure::Failed(format!("respuesta inválida de {method}: {e}")))
}

/// Los nombres de los calendarios, por identificador.
///
/// Si no se pueden leer, el tablero sigue con los eventos sin el nombre: no
/// vale la pena perder lo que sí llegó por un dato de adorno.
async fn calendar_names(connection: &Connection) -> HashMap<String, String> {
    match call_json(connection, "ListCalendars", &()).await {
        Ok(json) => serde_json::from_str::<Vec<CalendarItem>>(&json)
            .map(|items| items.into_iter().map(|c| (c.id, c.display_name)).collect())
            .unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

/// Junta las páginas de un rango y les pone el nombre de su calendario.
pub(crate) async fn read_occurrences(
    connection: &Connection,
    from: &str,
    to: &str,
) -> CalendarReply {
    let mut entries = Vec::new();
    let mut truncated = false;
    let mut cursor = String::new();

    for page_number in 0..MAX_PAGES {
        let json = match call_json(
            connection,
            "ListOccurrences",
            &(from, to, Vec::<String>::new(), cursor.as_str(), PAGE_LIMIT),
        )
        .await
        {
            Ok(json) => json,
            Err(failure) => return failure.into(),
        };
        let page: OccurrencePage = match serde_json::from_str(&json) {
            Ok(page) => page,
            Err(e) => {
                return CalendarReply::Failed {
                    detail: format!("no se pudo leer la página de eventos: {e}"),
                }
            }
        };
        truncated |= page.truncated;
        entries.extend(page.items);
        match page.next_cursor {
            Some(next) if !next.is_empty() => {
                cursor = next;
                if page_number + 1 == MAX_PAGES {
                    truncated = true;
                }
            }
            _ => break,
        }
    }

    let names = calendar_names(connection).await;
    for entry in &mut entries {
        entry.calendar = names.get(&entry.calendar_id).cloned();
    }

    CalendarReply::Ready { entries, truncated }
}

/// El lugar de cada evento pedido, en el mismo orden; vacío si no tiene o no se
/// pudo leer.
pub(crate) async fn read_locations(connection: &Connection, events: &[EventRef]) -> Vec<String> {
    let mut locations = Vec::with_capacity(events.len().min(MAX_DETAILS));
    for event in events.iter().take(MAX_DETAILS) {
        let location = call_json(
            connection,
            "GetEvent",
            &(event.event_id.as_str(), event.occurrence_id.as_str()),
        )
        .await
        .ok()
        .and_then(|json| {
            serde_json::from_str::<Option<EventDetail>>(&json)
                .ok()
                .flatten()
        })
        .map(|detail| detail.location.trim().to_string())
        .unwrap_or_default();
        locations.push(location);
    }
    locations
}

/// Un evento por el que se pregunta.
#[derive(Debug, Clone, Deserialize)]
pub struct EventRef {
    pub event_id: String,
    #[serde(default)]
    pub occurrence_id: String,
}

async fn session(app: &AppHandle) -> Option<Connection> {
    app.try_state::<DbusPool>()?.session().await
}

/// Las veces de los eventos que caen en `[from, to)`, en RFC 3339.
#[tauri::command]
pub async fn calendar_occurrences(app: AppHandle, from: String, to: String) -> CalendarReply {
    let Some(connection) = session(&app).await else {
        return CalendarReply::Unavailable;
    };
    let reply = read_occurrences(&connection, &from, &to).await;
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
    match session(&app).await {
        Some(connection) => read_locations(&connection, &events).await,
        None => vec![String::new(); events.len().min(MAX_DETAILS)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixStream;
    use std::sync::{Arc, Mutex};
    use zbus::{connection, Guid};

    /// Un almacén de mentira, del otro lado de un par de sockets: la llamada
    /// de verdad, con su encabezado y su cuerpo, sin tocar el bus de la sesión.
    #[derive(Clone, Default)]
    struct FakeStore {
        pages: Vec<String>,
        calendars: String,
        events: HashMap<String, String>,
        deny: bool,
        cursors: Arc<Mutex<Vec<String>>>,
    }

    #[zbus::interface(name = "ar.net.vasak.os.AccountsStore")]
    impl FakeStore {
        fn list_occurrences(
            &self,
            _from: String,
            _to: String,
            _calendar_ids: Vec<String>,
            cursor: String,
            _limit: u32,
        ) -> zbus::fdo::Result<String> {
            if self.deny {
                return Err(zbus::fdo::Error::AccessDenied("sin permiso".into()));
            }
            self.cursors.lock().unwrap().push(cursor.clone());
            let index: usize = if cursor.is_empty() {
                0
            } else {
                cursor.parse().unwrap()
            };
            Ok(self.pages[index].clone())
        }

        fn list_calendars(&self) -> String {
            self.calendars.clone()
        }

        fn get_event(&self, event_id: String, _occurrence_id: String) -> String {
            self.events
                .get(&event_id)
                .cloned()
                .unwrap_or_else(|| "null".into())
        }
    }

    async fn connect(store: Option<FakeStore>) -> Connection {
        let guid = Guid::generate();
        let (client, server) = UnixStream::pair().unwrap();
        let server = async move {
            let builder = connection::Builder::unix_stream(server).server(guid)?.p2p();
            // Sin almacén, el objeto se publica en otra ruta: así hay quien
            // conteste —«ese objeto no existe»— en vez de dejar la llamada
            // esperando.
            match store {
                Some(store) => builder.serve_at(PATH, store)?.build().await,
                None => {
                    builder
                        .serve_at("/otro", FakeStore::default())?
                        .build()
                        .await
                }
            }
        };
        let client = connection::Builder::unix_stream(client).p2p().build();
        let (client, server) = futures_util::try_join!(client, server).unwrap();
        // El servidor tiene que seguir vivo mientras dure la prueba.
        std::mem::forget(server);
        client
    }

    fn occurrence(id: &str, calendar: &str) -> String {
        format!(
            r##"{{"event_id":"{id}","occurrence_id":"","calendar_id":"{calendar}","title":"Evento {id}","start":"2026-03-23T13:00:00Z","end":"2026-03-23T14:00:00Z","all_day":false,"floating":false,"color":"#1e88e5"}}"##
        )
    }

    #[tokio::test]
    async fn junta_las_paginas_y_les_pone_el_nombre_del_calendario() {
        let store = FakeStore {
            pages: vec![
                format!(r#"{{"items":[{}],"next_cursor":"1","truncated":false}}"#, occurrence("a/1", "a/7")),
                format!(r#"{{"items":[{}],"next_cursor":null,"truncated":true}}"#, occurrence("a/2", "a/9")),
            ],
            calendars: r#"[{"id":"a/7","account_id":"a","display_name":"Trabajo","color":null,"components":["VEVENT"]}]"#.into(),
            ..Default::default()
        };
        let cursors = store.cursors.clone();
        let connection = connect(Some(store)).await;

        let reply =
            read_occurrences(&connection, "2026-03-01T00:00:00Z", "2026-04-12T00:00:00Z").await;

        let CalendarReply::Ready { entries, truncated } = reply else {
            panic!("se esperaban eventos: {reply:?}");
        };
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].calendar.as_deref(), Some("Trabajo"));
        // Un calendario que no está en la lista queda sin nombre, no rompe nada.
        assert_eq!(entries[1].calendar, None);
        assert!(truncated, "la segunda página venía recortada");
        assert_eq!(
            *cursors.lock().unwrap(),
            vec![String::new(), "1".to_string()]
        );
    }

    #[tokio::test]
    async fn un_permiso_negado_se_ve_como_negado() {
        let store = FakeStore {
            deny: true,
            ..Default::default()
        };
        let connection = connect(Some(store)).await;

        assert_eq!(
            read_occurrences(&connection, "a", "b").await,
            CalendarReply::Denied
        );
    }

    #[tokio::test]
    async fn sin_almacen_del_otro_lado_es_no_disponible() {
        // Un servicio sin el objeto: es lo que contesta una versión anterior al
        // almacén, y no es un error para la persona.
        let connection = connect(None).await;

        assert_eq!(
            read_occurrences(&connection, "a", "b").await,
            CalendarReply::Unavailable
        );
    }

    #[tokio::test]
    async fn el_lugar_sale_de_get_event_y_sin_evento_queda_vacio() {
        let mut events = HashMap::new();
        events.insert(
            "a/1".to_string(),
            r#"{"event_id":"a/1","location":"  Sala 3 "}"#.to_string(),
        );
        let store = FakeStore {
            events,
            ..Default::default()
        };
        let connection = connect(Some(store)).await;

        let refs = vec![
            EventRef {
                event_id: "a/1".into(),
                occurrence_id: String::new(),
            },
            EventRef {
                event_id: "a/2".into(),
                occurrence_id: "5".into(),
            },
        ];
        assert_eq!(
            read_locations(&connection, &refs).await,
            vec!["Sala 3".to_string(), String::new()]
        );
    }

    #[test]
    fn los_errores_de_dbus_caen_en_lo_que_ve_la_persona() {
        assert_eq!(
            classify_name("org.freedesktop.DBus.Error.ServiceUnknown"),
            Some(Failure::Unavailable)
        );
        assert_eq!(
            classify_name("org.freedesktop.DBus.Error.UnknownMethod"),
            Some(Failure::Unavailable)
        );
        assert_eq!(
            classify_name("org.freedesktop.DBus.Error.AccessDenied"),
            Some(Failure::Denied)
        );
        assert_eq!(classify_name("org.freedesktop.DBus.Error.Failed"), None);
    }

    #[test]
    fn la_respuesta_viaja_con_su_estado() {
        let ready = serde_json::to_value(CalendarReply::Ready {
            entries: vec![],
            truncated: false,
        })
        .unwrap();
        assert_eq!(ready["state"], "ready");
        assert_eq!(
            serde_json::to_value(CalendarReply::Denied).unwrap()["state"],
            "denied"
        );
        assert_eq!(
            serde_json::to_value(CalendarReply::Unavailable).unwrap()["state"],
            "unavailable"
        );
        let failed = serde_json::to_value(CalendarReply::Failed { detail: "x".into() }).unwrap();
        assert_eq!(failed["state"], "failed");
        assert_eq!(failed["detail"], "x");
    }

    #[test]
    fn la_espera_le_gana_al_dialogo_de_permiso() {
        assert!(CALL_TIMEOUT > Duration::from_secs(120));
    }
}
