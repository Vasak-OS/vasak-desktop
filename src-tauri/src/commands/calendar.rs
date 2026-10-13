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
//! error del plugin cae en una de ellas ([`classify_store`]).

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_vsk_store::{Error as StoreError, StoreClient, StoreExt};
#[cfg(test)]
use zbus::Connection;

use crate::logger::log_info;

#[cfg(test)]
const BUS_NAME: &str = "ar.net.vasak.os.AccountsSync";
#[cfg(test)]
const PATH: &str = "/ar/net/vasak/os/AccountsStore";
#[cfg(test)]
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
pub(crate) enum Failure {
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

#[cfg(test)]
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

/// Una llamada al almacén que contesta JSON, con el tope de espera. Sólo para
/// las pruebas: en la sesión, todo pasa por el cliente del plugin.
#[cfg(test)]
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

/// Lo que el tablero le pide al almacén.
pub(crate) enum Request<'a> {
    Calendars,
    Occurrences {
        from: &'a str,
        to: &'a str,
        cursor: &'a str,
    },
    Event {
        event_id: &'a str,
        occurrence_id: &'a str,
    },
}

impl Request<'_> {
    fn method(&self) -> &'static str {
        match self {
            Request::Calendars => "ListCalendars",
            Request::Occurrences { .. } => "ListOccurrences",
            Request::Event { .. } => "GetEvent",
        }
    }
}

/// Quién contesta los pedidos al almacén. En la sesión, el cliente de
/// `tauri-plugin-vsk-store`, que comprueba que el dueño del nombre del
/// sincronizador sea el de verdad antes de creerle nada; en las pruebas, una
/// conexión punto a punto con un almacén de mentira.
pub(crate) trait StoreCalls {
    fn json(
        &self,
        request: Request<'_>,
    ) -> impl std::future::Future<Output = Result<String, Failure>> + Send;
}

/// Una conexión punto a punto con un almacén de mentira, para las pruebas. Va
/// al nombre conocido sin comprobar al dueño: en la sesión no se usa nunca.
#[cfg(test)]
impl StoreCalls for Connection {
    async fn json(&self, request: Request<'_>) -> Result<String, Failure> {
        match request {
            Request::Calendars => call_json(self, "ListCalendars", &()).await,
            Request::Occurrences { from, to, cursor } => {
                call_json(
                    self,
                    "ListOccurrences",
                    &(from, to, Vec::<String>::new(), cursor, PAGE_LIMIT),
                )
                .await
            }
            Request::Event {
                event_id,
                occurrence_id,
            } => call_json(self, "GetEvent", &(event_id, occurrence_id)).await,
        }
    }
}

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

/// Lo que quiere decir un error del plugin para quien mira el tablero. Sin
/// sincronizador, o con uno que no es el de verdad, es «no disponible».
pub(crate) fn classify_store(error: StoreError) -> Failure {
    match &error {
        StoreError::NotRunning | StoreError::Impostor => Failure::Unavailable,
        StoreError::Store {
            name: Some(name),
            message,
        } => classify_name(name).unwrap_or_else(|| Failure::Failed(message.clone())),
        _ => Failure::Failed(error.to_string()),
    }
}

/// Los nombres de los calendarios, por identificador.
///
/// Si no se pueden leer, el tablero sigue con los eventos sin el nombre: no
/// vale la pena perder lo que sí llegó por un dato de adorno.
async fn calendar_names(store: &impl StoreCalls) -> HashMap<String, String> {
    match store.json(Request::Calendars).await {
        Ok(json) => serde_json::from_str::<Vec<CalendarItem>>(&json)
            .map(|items| items.into_iter().map(|c| (c.id, c.display_name)).collect())
            .unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

/// Junta las páginas de un rango y les pone el nombre de su calendario.
pub(crate) async fn read_occurrences(
    store: &impl StoreCalls,
    from: &str,
    to: &str,
) -> CalendarReply {
    let mut entries = Vec::new();
    let mut truncated = false;
    let mut cursor = String::new();

    for page_number in 0..MAX_PAGES {
        let json = match store
            .json(Request::Occurrences {
                from,
                to,
                cursor: cursor.as_str(),
            })
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

    let names = calendar_names(store).await;
    for entry in &mut entries {
        entry.calendar = names.get(&entry.calendar_id).cloned();
    }

    CalendarReply::Ready { entries, truncated }
}

/// El lugar de cada evento pedido, en el mismo orden; vacío si no tiene o no se
/// pudo leer.
pub(crate) async fn read_locations(store: &impl StoreCalls, events: &[EventRef]) -> Vec<String> {
    let mut locations = Vec::with_capacity(events.len().min(MAX_DETAILS));
    for event in events.iter().take(MAX_DETAILS) {
        let location = store
            .json(Request::Event {
                event_id: event.event_id.as_str(),
                occurrence_id: event.occurrence_id.as_str(),
            })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    // De tokio y no de `std`: con `tauri-plugin-display-manager` zbus se
    // compila con su característica `tokio`, y entonces `unix_stream` pide
    // este tipo.
    use tokio::net::UnixStream;
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
        /// Cada pedido, con sus argumentos y en orden.
        calls: Arc<Mutex<Vec<String>>>,
    }

    #[zbus::interface(name = "ar.net.vasak.os.AccountsStore")]
    impl FakeStore {
        fn list_occurrences(
            &self,
            from: String,
            to: String,
            calendar_ids: Vec<String>,
            cursor: String,
            limit: u32,
        ) -> zbus::fdo::Result<String> {
            self.calls.lock().unwrap().push(format!(
                "ListOccurrences {from} {to} {calendar_ids:?} {cursor:?} {limit}"
            ));
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
            self.calls.lock().unwrap().push("ListCalendars".into());
            self.calendars.clone()
        }

        fn get_event(&self, event_id: String, occurrence_id: String) -> String {
            self.calls
                .lock()
                .unwrap()
                .push(format!("GetEvent {event_id} {occurrence_id:?}"));
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

    // ── Los errores del plugin ─────────────────────────────────────────────

    fn store_error(name: Option<&str>, message: &str) -> StoreError {
        StoreError::Store {
            name: name.map(str::to_string),
            message: message.into(),
        }
    }

    /// Sin sincronizador, o con uno que no es el de verdad, el tablero dice
    /// «no disponible», nunca «falló»: no hay nada roto que la persona pueda
    /// arreglar.
    #[test]
    fn sin_sincronizador_o_con_un_impostor_el_tablero_ve_no_disponible() {
        assert_eq!(classify_store(StoreError::NotRunning), Failure::Unavailable);
        assert_eq!(classify_store(StoreError::Impostor), Failure::Unavailable);
    }

    #[test]
    fn los_errores_de_dbus_del_plugin_caen_en_lo_que_ve_la_persona() {
        for name in [
            "org.freedesktop.DBus.Error.AccessDenied",
            "org.freedesktop.DBus.Error.AuthFailed",
        ] {
            assert_eq!(
                classify_store(store_error(Some(name), "x")),
                Failure::Denied,
                "{name}"
            );
        }
        for name in [
            "org.freedesktop.DBus.Error.ServiceUnknown",
            "org.freedesktop.DBus.Error.NameHasNoOwner",
            "org.freedesktop.DBus.Error.UnknownObject",
            "org.freedesktop.DBus.Error.UnknownMethod",
        ] {
            assert_eq!(
                classify_store(store_error(Some(name), "x")),
                Failure::Unavailable,
                "{name}"
            );
        }
    }

    /// Lo que no es ni permiso ni servicio ausente es un fallo, con el texto
    /// del almacén y no el nombre técnico del error.
    #[test]
    fn los_demas_errores_del_plugin_son_un_fallo_con_su_texto() {
        assert_eq!(
            classify_store(store_error(
                Some("org.freedesktop.DBus.Error.Failed"),
                "la base está cerrada"
            )),
            Failure::Failed("la base está cerrada".into())
        );
        assert_eq!(
            classify_store(store_error(None, "se cortó")),
            Failure::Failed("se cortó".into())
        );
        let Failure::Failed(detail) = classify_store(StoreError::Reply("rota".into())) else {
            panic!("una respuesta rota es un fallo");
        };
        assert!(detail.contains("rota"), "{detail}");
    }

    // ── Por el cliente comprobado del plugin ───────────────────────────────
    //
    // Las pruebas de arriba usan una conexión punto a punto, que en la sesión
    // no se usa nunca. Estas pasan por `StoreCalls for StoreClient`, lo que usa
    // el escritorio de verdad, contra un `dbus-daemon` privado: el almacén de
    // mentira se sirve con el zbus 4 del escritorio y el plugin le habla con el
    // suyo.

    mod bus {
        use super::*;
        use std::io::{BufRead, BufReader};
        use std::process::{Child, Command, Stdio};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tauri_plugin_vsk_store::client::{OwnerCheck, SERVICE};

        static LOCK: Mutex<()> = Mutex::new(());
        static NEXT: AtomicUsize = AtomicUsize::new(0);

        /// Un `dbus-daemon` sólo para una prueba, sin activación de servicios:
        /// la del sistema arrancaría el sincronizador de verdad. El cliente del
        /// plugin abre el bus **de sesión**, así que la prueba apunta ahí
        /// `DBUS_SESSION_BUS_ADDRESS`; con el candado, de a una.
        struct PrivateBus {
            child: Child,
            config: std::path::PathBuf,
            address: String,
            _turn: std::sync::MutexGuard<'static, ()>,
        }

        impl PrivateBus {
            fn start() -> Self {
                let turn = LOCK.lock().unwrap_or_else(|e| e.into_inner());
                let dir = std::env::temp_dir();
                let config = dir.join(format!(
                    "vsk-desktop-test-{}-{}.conf",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::SeqCst)
                ));
                std::fs::write(
                    &config,
                    format!(
                        r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:tmpdir={}</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#,
                        dir.display()
                    ),
                )
                .unwrap();
                let mut child = Command::new("dbus-daemon")
                    .arg(format!("--config-file={}", config.display()))
                    .arg("--nofork")
                    .arg("--print-address")
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect("hace falta `dbus-daemon` para estas pruebas (paquete dbus-daemon)");
                let mut address = String::new();
                BufReader::new(child.stdout.take().unwrap())
                    .read_line(&mut address)
                    .unwrap();
                let address = address.trim().to_string();
                assert!(!address.is_empty(), "dbus-daemon no dio su dirección");
                std::env::set_var("DBUS_SESSION_BUS_ADDRESS", &address);
                Self {
                    child,
                    config,
                    address,
                    _turn: turn,
                }
            }

            /// Publica el almacén de mentira y toma el nombre del sincronizador.
            async fn serve(&self, store: FakeStore) -> Connection {
                connection::Builder::address(self.address.as_str())
                    .unwrap()
                    .serve_at(PATH, store)
                    .unwrap()
                    .name(SERVICE)
                    .unwrap()
                    .build()
                    .await
                    .unwrap()
            }
        }

        impl Drop for PrivateBus {
            fn drop(&mut self) {
                let _ = self.child.kill();
                let _ = self.child.wait();
                let _ = std::fs::remove_file(&self.config);
            }
        }

        /// La comprobación del dueño, decidida por la prueba: el almacén de
        /// mentira corre en este proceso, que no es el sincronizador instalado.
        struct Decided(bool);

        impl OwnerCheck for Decided {
            fn trusted(&self, _pid: u32) -> bool {
                self.0
            }
        }

        fn client(trusted: bool) -> StoreClient {
            StoreClient::with_check(std::sync::Arc::new(Decided(trusted)))
        }

        const FROM: &str = "2026-03-01T00:00:00Z";
        const TO: &str = "2026-04-12T00:00:00Z";

        /// Cada pedido del tablero llega al almacén con su método y sus
        /// argumentos: el rango en orden, todos los calendarios, el cursor de
        /// cada página y el tamaño de página; el lugar, por evento y vez.
        #[tokio::test]
        async fn el_cliente_del_plugin_pide_cada_cosa_con_su_metodo_y_sus_argumentos() {
            let bus = PrivateBus::start();
            let mut events = HashMap::new();
            events.insert(
                "a/1".to_string(),
                r#"{"event_id":"a/1","location":"Sala 3"}"#.to_string(),
            );
            let store = FakeStore {
                pages: vec![
                    format!(r#"{{"items":[{}],"next_cursor":"1","truncated":false}}"#, occurrence("a/1", "a/7")),
                    format!(r#"{{"items":[{}],"next_cursor":null,"truncated":false}}"#, occurrence("a/2", "a/7")),
                ],
                calendars: r#"[{"id":"a/7","account_id":"a","display_name":"Trabajo","color":null,"components":["VEVENT"]}]"#.into(),
                events,
                ..Default::default()
            };
            let calls = store.calls.clone();
            let _servicio = bus.serve(store).await;
            let client = client(true);

            let reply = read_occurrences(&client, FROM, TO).await;
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
            let locations = read_locations(&client, &refs).await;

            let CalendarReply::Ready { entries, truncated } = reply else {
                panic!("se esperaban eventos: {reply:?}");
            };
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].calendar.as_deref(), Some("Trabajo"));
            assert!(!truncated);
            assert_eq!(locations, vec!["Sala 3".to_string(), String::new()]);
            assert_eq!(
                *calls.lock().unwrap(),
                vec![
                    format!("ListOccurrences {FROM} {TO} [] \"\" {PAGE_LIMIT}"),
                    format!("ListOccurrences {FROM} {TO} [] \"1\" {PAGE_LIMIT}"),
                    "ListCalendars".to_string(),
                    "GetEvent a/1 \"\"".to_string(),
                    "GetEvent a/2 \"5\"".to_string(),
                ]
            );
        }

        #[tokio::test]
        async fn por_el_cliente_del_plugin_un_permiso_negado_se_ve_como_negado() {
            let bus = PrivateBus::start();
            let _servicio = bus
                .serve(FakeStore {
                    deny: true,
                    ..Default::default()
                })
                .await;

            assert_eq!(
                read_occurrences(&client(true), FROM, TO).await,
                CalendarReply::Denied
            );
        }

        /// A un impostor no se le pide nada —ni los eventos ni el lugar— y el
        /// tablero lo ve como un servicio que no está.
        #[tokio::test]
        async fn a_un_impostor_no_se_le_pide_nada_y_el_tablero_ve_no_disponible() {
            let bus = PrivateBus::start();
            let store = FakeStore::default();
            let calls = store.calls.clone();
            let _impostor = bus.serve(store).await;
            let client = client(false);

            let reply = read_occurrences(&client, FROM, TO).await;
            let locations = read_locations(
                &client,
                &[EventRef {
                    event_id: "a/1".into(),
                    occurrence_id: String::new(),
                }],
            )
            .await;

            assert_eq!(reply, CalendarReply::Unavailable);
            assert_eq!(locations, vec![String::new()]);
            assert!(
                calls.lock().unwrap().is_empty(),
                "{:?}",
                calls.lock().unwrap()
            );
        }

        #[tokio::test]
        async fn sin_sincronizador_en_el_bus_el_tablero_ve_no_disponible() {
            let _bus = PrivateBus::start();

            assert_eq!(
                read_occurrences(&client(true), FROM, TO).await,
                CalendarReply::Unavailable
            );
        }
    }
}
