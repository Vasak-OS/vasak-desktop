//! Progreso, contador y urgencia de las aplicaciones:
//! `com.canonical.Unity.LauncherEntry`.
//!
//! StatusNotifierItem no tiene ningún campo de progreso ni de contador. Lo que
//! usan las aplicaciones para eso es la API del lanzador de Unity, que **no es
//! una interfaz que se consulta sino una señal que se emite**:
//!
//! ```text
//! signal com.canonical.Unity.LauncherEntry.Update (s app_uri, a{sv} properties)
//! ```
//!
//! - `app_uri` es `application://<id del .desktop>`, con la extensión
//!   (`application://firefox.desktop`).
//! - `properties` trae **sólo lo que cambió**: `count` (`x`), `count-visible`
//!   (`b`), `progress` (`d`, de 0 a 1), `progress-visible` (`b`), `urgent`
//!   (`b`) y `quicklist` (`s`, que acá no se usa). El contador y el progreso
//!   se dibujan sólo con su `*-visible` en verdadero.
//!
//! Se escucha la señal de cualquier emisor, como hacía Unity. No hay registro:
//! quien emite es quien es. Qt 6 la emite sola desde `setBadgeNumber` —Telegram
//! la usa para los no leídos—, Electron 44 en adelante desde `setBadgeCount` y
//! `setProgressBar`, y las versiones anteriores de Electron sólo si encuentran
//! `libunity.so.9` instalada.
//!
//! # A qué elemento de la bandeja le toca
//!
//! La señal dice de qué `.desktop` es, y un elemento de la bandeja no dice de
//! qué `.desktop` es (su `Id` es libre: `TelegramDesktop`,
//! `discord_status_icon_1`). Se asocian, en este orden:
//!
//! 1. **Por proceso**: el pid de quien emitió la señal es el del dueño del
//!    elemento. Es lo más firme, y cubre a Chromium y Electron, que usan una
//!    conexión al bus para cada cosa pero desde el mismo proceso.
//! 2. **Por conexión**: el mismo nombre único en el bus.
//! 3. **Por nombre**: el id del `.desktop` normalizado contra el `Id`, el
//!    título o el título del globo del elemento (`org.telegram.desktop` y
//!    `TelegramDesktop` dan los dos `telegramdesktop`).
//!
//! Una entrada sin elemento se guarda igual: la aplicación puede publicar su
//! icono en la bandeja después, o tener sólo una ventana en el panel.

use crate::logger::{log_debug, log_warning};
use crate::structs::{LauncherBadge, TrayItem, TrayManager};
use futures_util::stream::StreamExt;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{async_runtime::RwLock, AppHandle, Emitter};
use zbus::zvariant::{OwnedValue, Value};
use zbus::{Connection, MatchRule, MessageStream, MessageType};

pub const LAUNCHER_ENTRY_INTERFACE: &str = "com.canonical.Unity.LauncherEntry";

/// Lo último que dijo una aplicación, acumulado: cada `Update` trae sólo lo que
/// cambió.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LauncherEntryState {
    pub count: Option<i64>,
    pub count_visible: bool,
    pub progress: Option<f64>,
    pub progress_visible: bool,
    pub urgent: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LauncherEntry {
    /// El nombre único de quien emitió la última señal.
    pub sender: String,
    pub pid: Option<u32>,
    pub state: LauncherEntryState,
}

/// Las entradas conocidas, por `app_uri`.
pub type LauncherEntryStore = Arc<RwLock<HashMap<String, LauncherEntry>>>;

pub fn create_launcher_entry_store() -> LauncherEntryStore {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Lo que se le manda al panel para las ventanas: sólo lo visible.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LauncherEntryView {
    pub desktop_id: String,
    #[serde(flatten)]
    pub badge: LauncherBadge,
}

fn unwrap_variant<'a>(v: &'a Value<'a>) -> &'a Value<'a> {
    match v {
        Value::Value(inner) => unwrap_variant(inner),
        other => other,
    }
}

fn as_i64(v: &Value) -> Option<i64> {
    match unwrap_variant(v) {
        Value::I64(n) => Some(*n),
        Value::I32(n) => Some(*n as i64),
        Value::U32(n) => Some(*n as i64),
        Value::U64(n) => i64::try_from(*n).ok(),
        Value::I16(n) => Some(*n as i64),
        Value::U16(n) => Some(*n as i64),
        Value::U8(n) => Some(*n as i64),
        _ => None,
    }
}

fn as_f64(v: &Value) -> Option<f64> {
    match unwrap_variant(v) {
        Value::F64(n) => Some(*n),
        other => as_i64(other).map(|n| n as f64),
    }
}

fn as_bool(v: &Value) -> Option<bool> {
    match unwrap_variant(v) {
        Value::Bool(b) => Some(*b),
        _ => None,
    }
}

/// `application://firefox.desktop` → `firefox`. Cualquier otra forma no es
/// una entrada del lanzador.
pub fn desktop_id_from_uri(app_uri: &str) -> Option<String> {
    let id = app_uri.strip_prefix("application://")?;
    let id = id.strip_suffix(".desktop").unwrap_or(id);
    (!id.trim().is_empty()).then(|| id.to_string())
}

/// Aplica un `Update`. Una propiedad con el tipo equivocado se ignora, como si
/// no hubiera venido: no borra lo que ya se sabía.
pub fn apply_update<'a, I>(state: &mut LauncherEntryState, properties: I)
where
    I: IntoIterator<Item = (&'a str, &'a Value<'a>)>,
{
    for (key, value) in properties {
        match key {
            "count" => {
                if let Some(count) = as_i64(value) {
                    state.count = Some(count);
                }
            }
            "count-visible" => {
                if let Some(visible) = as_bool(value) {
                    state.count_visible = visible;
                }
            }
            "progress" => {
                if let Some(progress) = as_f64(value) {
                    state.progress = Some(progress);
                }
            }
            "progress-visible" => {
                if let Some(visible) = as_bool(value) {
                    state.progress_visible = visible;
                }
            }
            "urgent" => {
                if let Some(urgent) = as_bool(value) {
                    state.urgent = urgent;
                }
            }
            _ => {}
        }
    }
}

/// Lo que se dibuja de una entrada, o `None` si no hay nada que dibujar.
///
/// - El contador, con `count-visible`, si es mayor que cero: un «0» encima de
///   un icono no informa nada, y es lo que manda Qt al quedar todo leído (con
///   `count-visible` en falso, igual).
/// - El progreso, con `progress-visible`, recortado a [0, 1]: la especificación
///   dice «entre 0 y 1» y hay quien manda 1,02 al terminar. Un `NaN` no se
///   dibuja.
pub fn badge_of(state: &LauncherEntryState) -> Option<LauncherBadge> {
    let count = state
        .count
        .filter(|count| state.count_visible && *count > 0);
    let progress = state
        .progress
        .filter(|p| state.progress_visible && p.is_finite())
        .map(|p| p.clamp(0.0, 1.0));
    let badge = LauncherBadge {
        count,
        progress,
        urgent: state.urgent,
    };
    (badge != LauncherBadge::default()).then_some(badge)
}

/// Prefijos de dominio invertido que no dicen nada del nombre.
const DOMAIN_PREFIXES: &[&str] = &[
    "org", "com", "io", "net", "dev", "app", "me", "de", "fr", "es", "ar",
];

/// Palabras que como último segmento no identifican a nadie.
const GENERIC_SEGMENTS: &[&str] = &["desktop", "app", "client", "application", "bin"];

fn squash(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Los nombres con los que se puede reconocer un `.desktop`.
fn desktop_names(desktop_id: &str) -> Vec<String> {
    let lower = desktop_id.to_lowercase();
    let mut segments: Vec<&str> = lower.split('.').filter(|s| !s.is_empty()).collect();
    let mut names = Vec::new();
    if segments.len() > 1 && DOMAIN_PREFIXES.contains(&segments[0]) {
        segments.remove(0);
    }
    names.push(squash(&segments.join("")));
    if let Some(last) = segments.last() {
        if segments.len() > 1 && !GENERIC_SEGMENTS.contains(last) {
            names.push(squash(last));
        }
    }
    names.retain(|n| !n.is_empty());
    names
}

/// Los nombres con los que se puede reconocer un elemento de la bandeja.
fn item_names(item: &TrayItem) -> Vec<String> {
    let mut names = Vec::new();
    let mut push = |raw: &str| {
        // Chromium y Electron llaman a sus elementos `<app>_status_icon_<n>`.
        let raw = match raw.find("_status_icon") {
            Some(cut) => &raw[..cut],
            None => raw,
        };
        let name = squash(raw);
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    };
    push(&item.id);
    if let Some(title) = item.title.as_deref() {
        push(title);
    }
    if let Some(title) = item.tooltip.as_ref().and_then(|t| t.title.as_deref()) {
        push(title);
    }
    names
}

/// La entrada que le corresponde a un elemento, si hay alguna. Ver el
/// comentario del módulo para el orden.
pub fn entry_for_item<'a>(
    item: &TrayItem,
    entries: &'a HashMap<String, LauncherEntry>,
) -> Option<&'a LauncherEntry> {
    if let Some(pid) = item.pid {
        if let Some(entry) = entries.values().find(|e| e.pid == Some(pid)) {
            return Some(entry);
        }
    }
    if let Some(unique) = item.unique_name.as_deref() {
        if let Some(entry) = entries.values().find(|e| e.sender == unique) {
            return Some(entry);
        }
    }
    let names = item_names(item);
    entries.iter().find_map(|(uri, entry)| {
        let id = desktop_id_from_uri(uri)?;
        desktop_names(&id)
            .iter()
            .any(|n| names.contains(n))
            .then_some(entry)
    })
}

/// Le pone a cada elemento lo que dice su entrada. Devuelve si cambió algo,
/// para avisarle a la vista sólo entonces.
pub fn attach_badges(
    items: &mut HashMap<String, TrayItem>,
    entries: &HashMap<String, LauncherEntry>,
) -> bool {
    let mut changed = false;
    for item in items.values_mut() {
        let badge = entry_for_item(item, entries).and_then(|e| badge_of(&e.state));
        if item.launcher != badge {
            item.launcher = badge;
            changed = true;
        }
    }
    changed
}

/// Las entradas con algo que dibujar, para las ventanas del panel.
pub fn visible_entries(entries: &HashMap<String, LauncherEntry>) -> Vec<LauncherEntryView> {
    let mut views: Vec<LauncherEntryView> = entries
        .iter()
        .filter_map(|(uri, entry)| {
            Some(LauncherEntryView {
                desktop_id: desktop_id_from_uri(uri)?,
                badge: badge_of(&entry.state)?,
            })
        })
        .collect();
    views.sort_by(|a, b| a.desktop_id.cmp(&b.desktop_id));
    views
}

/// Aplica una señal `Update` a las entradas. Devuelve si cambió lo que se
/// dibuja (una entrada que sigue sin nada visible no cambia nada).
pub fn record_update(
    entries: &mut HashMap<String, LauncherEntry>,
    app_uri: &str,
    sender: &str,
    pid: Option<u32>,
    properties: &HashMap<String, OwnedValue>,
) -> bool {
    if desktop_id_from_uri(app_uri).is_none() {
        return false;
    }
    let entry = entries
        .entry(app_uri.to_string())
        .or_insert_with(|| LauncherEntry {
            sender: sender.to_string(),
            pid,
            state: LauncherEntryState::default(),
        });
    let before = (badge_of(&entry.state), entry.pid);
    entry.sender = sender.to_string();
    entry.pid = pid.or(entry.pid);
    apply_update(
        &mut entry.state,
        properties.iter().map(|(k, v)| (k.as_str(), &**v)),
    );
    before != (badge_of(&entry.state), entry.pid)
}

/// Se olvida de lo que publicó una conexión que se fue del bus. Devuelve si
/// había algo.
pub fn forget_sender(entries: &mut HashMap<String, LauncherEntry>, sender: &str) -> bool {
    let before = entries.len();
    entries.retain(|_, e| e.sender != sender);
    before != entries.len()
}

/// El proceso dueño de un nombre del bus.
pub async fn connection_pid(connection: &Connection, name: &str) -> Option<u32> {
    let proxy = zbus::fdo::DBusProxy::new(connection).await.ok()?;
    let name = zbus::names::BusName::try_from(name).ok()?;
    proxy.get_connection_unix_process_id(name).await.ok()
}

/// El nombre único del dueño de un nombre del bus (o él mismo si ya lo es).
pub async fn unique_name_of(connection: &Connection, name: &str) -> Option<String> {
    if name.starts_with(':') {
        return Some(name.to_string());
    }
    let proxy = zbus::fdo::DBusProxy::new(connection).await.ok()?;
    let name = zbus::names::BusName::try_from(name).ok()?;
    proxy
        .get_name_owner(name)
        .await
        .ok()
        .map(|owner| owner.to_string())
}

/// Les pone a los elementos lo que dicen las entradas y avisa a la vista.
pub async fn publish(
    app_handle: &AppHandle,
    tray_manager: &TrayManager,
    store: &LauncherEntryStore,
) {
    let entries = store.read().await;
    let changed = {
        let mut items = tray_manager.write().await;
        attach_badges(&mut items, &entries)
    };
    if changed {
        crate::tray::emit_tray_update(app_handle).await;
    }
    if let Err(e) = app_handle.emit("launcher-entry-update", visible_entries(&entries)) {
        log_warning(&format!(
            "[LauncherEntry] No se pudo avisar a la vista: {e}"
        ));
    }
}

/// Aplica una señal `Update` recibida. Devuelve si cambió lo que se dibuja.
pub async fn record_signal(
    connection: &Connection,
    store: &LauncherEntryStore,
    message: &zbus::Message,
) -> bool {
    let header = message.header();
    let Some(sender) = header.sender().map(|s| s.to_string()) else {
        return false;
    };
    let body = message.body();
    let Ok((app_uri, properties)) = body.deserialize::<(String, HashMap<String, OwnedValue>)>()
    else {
        log_debug("[LauncherEntry] Update con una firma que no es (sa{sv})");
        return false;
    };
    let pid = connection_pid(connection, &sender).await;
    let mut entries = store.write().await;
    record_update(&mut entries, &app_uri, &sender, pid, &properties)
}

/// Escucha `Update` de cualquier emisor y la salida de los emisores del bus.
pub async fn start(
    connection: Connection,
    tray_manager: TrayManager,
    store: LauncherEntryStore,
    app_handle: AppHandle,
) -> zbus::Result<()> {
    let update_rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface(LAUNCHER_ENTRY_INTERFACE)?
        .member("Update")?
        .build();
    let mut updates = MessageStream::for_match_rule(update_rule, &connection, None).await?;

    let owner_rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface("org.freedesktop.DBus")?
        .member("NameOwnerChanged")?
        .build();
    let mut owners = MessageStream::for_match_rule(owner_rule, &connection, None).await?;

    tokio::spawn({
        let connection = connection.clone();
        let tray_manager = tray_manager.clone();
        let store = store.clone();
        let app_handle = app_handle.clone();
        async move {
            // Un error suelto del flujo no lo corta: sólo `None` es el final.
            while let Some(message) = updates.next().await {
                let Ok(message) = message else {
                    log_debug("[LauncherEntry] Error en el flujo de Update; se sigue");
                    continue;
                };
                if record_signal(&connection, &store, &message).await {
                    publish(&app_handle, &tray_manager, &store).await;
                }
            }
        }
    });

    tokio::spawn(async move {
        while let Some(message) = owners.next().await {
            let Ok(message) = message else { continue };
            let body = message.body();
            let Ok((name, _old, new_owner)) = body.deserialize::<(&str, &str, &str)>() else {
                continue;
            };
            if !new_owner.is_empty() || !name.starts_with(':') {
                continue;
            }
            let gone = {
                let mut entries = store.write().await;
                forget_sender(&mut entries, name)
            };
            if gone {
                publish(&app_handle, &tray_manager, &store).await;
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structs::{TrayCategory, TrayStatus, TrayTooltip};

    fn props(list: Vec<(&str, Value<'static>)>) -> HashMap<String, OwnedValue> {
        list.into_iter()
            .map(|(k, v)| (k.to_string(), OwnedValue::try_from(v).unwrap()))
            .collect()
    }

    fn item(id: &str) -> TrayItem {
        TrayItem {
            id: id.to_string(),
            service_name: format!("org.kde.StatusNotifierItem-1-{id}"),
            bus_name: None,
            icon_name: None,
            icon_data: None,
            overlay_icon: None,
            attention_icon: None,
            attention_movie_name: None,
            title: None,
            tooltip: None,
            status: TrayStatus::Active,
            category: TrayCategory::ApplicationStatus,
            menu_path: None,
            item_is_menu: false,
            launcher: None,
            object_path: "/StatusNotifierItem".into(),
            unique_name: None,
            pid: None,
        }
    }

    #[test]
    fn el_uri_tiene_que_ser_de_una_aplicacion() {
        assert_eq!(
            desktop_id_from_uri("application://firefox.desktop").as_deref(),
            Some("firefox")
        );
        assert_eq!(
            desktop_id_from_uri("application://org.telegram.desktop.desktop").as_deref(),
            Some("org.telegram.desktop")
        );
        assert_eq!(desktop_id_from_uri("file:///x.desktop"), None);
        assert_eq!(desktop_id_from_uri("application://"), None);
    }

    #[test]
    fn sin_visible_no_se_dibuja_nada() {
        let mut state = LauncherEntryState::default();
        let p = props(vec![
            ("count", Value::I64(3)),
            ("progress", Value::F64(0.5)),
        ]);
        apply_update(&mut state, p.iter().map(|(k, v)| (k.as_str(), &**v)));
        assert_eq!(badge_of(&state), None);
    }

    #[test]
    fn cada_update_trae_solo_lo_que_cambio() {
        let mut entries = HashMap::new();
        let uri = "application://firefox.desktop";
        record_update(
            &mut entries,
            uri,
            ":1.5",
            Some(10),
            &props(vec![
                ("progress", Value::F64(0.2)),
                ("progress-visible", Value::Bool(true)),
            ]),
        );
        // El siguiente sólo trae el progreso: la visibilidad sigue.
        assert!(record_update(
            &mut entries,
            uri,
            ":1.5",
            Some(10),
            &props(vec![("progress", Value::F64(0.7))])
        ));
        assert_eq!(badge_of(&entries[uri].state).unwrap().progress, Some(0.7));
        // Repetir lo mismo no cambia nada.
        assert!(!record_update(
            &mut entries,
            uri,
            ":1.5",
            Some(10),
            &props(vec![("progress", Value::F64(0.7))])
        ));
    }

    #[test]
    fn el_progreso_fuera_de_rango_se_recorta_y_nan_no_se_dibuja() {
        let state = |p: f64| LauncherEntryState {
            progress: Some(p),
            progress_visible: true,
            ..Default::default()
        };
        assert_eq!(badge_of(&state(-0.5)).unwrap().progress, Some(0.0));
        assert_eq!(badge_of(&state(1.7)).unwrap().progress, Some(1.0));
        assert_eq!(badge_of(&state(f64::NAN)), None);
        assert_eq!(badge_of(&state(f64::INFINITY)), None);
    }

    #[test]
    fn el_contador_en_cero_o_negativo_no_se_dibuja() {
        let state = |c: i64| LauncherEntryState {
            count: Some(c),
            count_visible: true,
            ..Default::default()
        };
        assert_eq!(badge_of(&state(0)), None);
        assert_eq!(badge_of(&state(-4)), None);
        assert_eq!(badge_of(&state(12)).unwrap().count, Some(12));
    }

    #[test]
    fn urgente_solo_tambien_es_algo_que_dibujar() {
        let state = LauncherEntryState {
            urgent: true,
            ..Default::default()
        };
        assert_eq!(
            badge_of(&state),
            Some(LauncherBadge {
                count: None,
                progress: None,
                urgent: true
            })
        );
    }

    #[test]
    fn un_tipo_equivocado_no_borra_lo_que_se_sabia() {
        let mut state = LauncherEntryState {
            count: Some(5),
            count_visible: true,
            ..Default::default()
        };
        let p = props(vec![
            ("count", Value::from("cinco")),
            ("count-visible", Value::from(1i32)),
        ]);
        apply_update(&mut state, p.iter().map(|(k, v)| (k.as_str(), &**v)));
        assert_eq!((state.count, state.count_visible), (Some(5), true));
        // Un contador en `i` (y no en `x`) se acepta igual.
        let p = props(vec![("count", Value::I32(7))]);
        apply_update(&mut state, p.iter().map(|(k, v)| (k.as_str(), &**v)));
        assert_eq!(state.count, Some(7));
    }

    #[test]
    fn un_uri_que_no_es_de_una_aplicacion_no_se_guarda() {
        let mut entries = HashMap::new();
        assert!(!record_update(
            &mut entries,
            "algo",
            ":1.5",
            None,
            &props(vec![("urgent", Value::Bool(true))])
        ));
        assert!(entries.is_empty());
    }

    fn entry(sender: &str, pid: Option<u32>, count: i64) -> LauncherEntry {
        LauncherEntry {
            sender: sender.into(),
            pid,
            state: LauncherEntryState {
                count: Some(count),
                count_visible: true,
                ..Default::default()
            },
        }
    }

    #[test]
    fn se_asocia_primero_por_proceso() {
        let mut entries = HashMap::new();
        entries.insert(
            "application://otra.desktop".into(),
            entry(":1.9", Some(42), 3),
        );
        entries.insert(
            "application://discord.desktop".into(),
            entry(":1.8", Some(7), 1),
        );
        let mut discord = item("discord_status_icon_1");
        discord.pid = Some(42);
        // Por proceso gana aunque el nombre diga otra cosa.
        assert_eq!(entry_for_item(&discord, &entries).unwrap().sender, ":1.9");
    }

    #[test]
    fn despues_por_conexion_y_despues_por_nombre() {
        let mut entries = HashMap::new();
        entries.insert(
            "application://org.telegram.desktop.desktop".into(),
            entry(":1.20", None, 4),
        );
        entries.insert(
            "application://discord.desktop".into(),
            entry(":1.30", None, 2),
        );

        let mut by_connection = item("cualquiera");
        by_connection.unique_name = Some(":1.30".into());
        assert_eq!(
            entry_for_item(&by_connection, &entries).unwrap().sender,
            ":1.30"
        );

        // Qt llama al elemento por el nombre de la aplicación.
        assert_eq!(
            entry_for_item(&item("TelegramDesktop"), &entries)
                .unwrap()
                .sender,
            ":1.20"
        );
        // Chromium y Electron, `<app>_status_icon_<n>`.
        assert_eq!(
            entry_for_item(&item("discord_status_icon_1"), &entries)
                .unwrap()
                .sender,
            ":1.30"
        );
        // Y el título del globo, si el id no dice nada.
        let mut by_tooltip = item("chrome_status_icon_1");
        by_tooltip.tooltip = Some(TrayTooltip {
            title: Some("Discord".into()),
            ..Default::default()
        });
        assert_eq!(
            entry_for_item(&by_tooltip, &entries).unwrap().sender,
            ":1.30"
        );
    }

    #[test]
    fn un_segmento_generico_no_asocia_a_cualquiera() {
        let mut entries = HashMap::new();
        entries.insert(
            "application://org.telegram.desktop.desktop".into(),
            entry(":1.20", None, 4),
        );
        // «desktop» es el último segmento de Telegram, pero no es Telegram.
        assert!(entry_for_item(&item("desktop"), &entries).is_none());
    }

    #[test]
    fn una_entrada_sin_elemento_no_le_cae_a_nadie_y_queda_para_las_ventanas() {
        let mut entries = HashMap::new();
        entries.insert(
            "application://firefox.desktop".into(),
            entry(":1.40", Some(99), 2),
        );
        let mut items = HashMap::new();
        items.insert("a".to_string(), item("nm-applet"));
        assert!(!attach_badges(&mut items, &entries));
        assert_eq!(items["a"].launcher, None);
        assert_eq!(
            visible_entries(&entries),
            vec![LauncherEntryView {
                desktop_id: "firefox".into(),
                badge: LauncherBadge {
                    count: Some(2),
                    progress: None,
                    urgent: false
                }
            }]
        );
    }

    #[test]
    fn el_elemento_recibe_y_pierde_su_insignia() {
        let mut entries = HashMap::new();
        entries.insert(
            "application://discord.desktop".into(),
            entry(":1.8", None, 5),
        );
        let mut items = HashMap::new();
        items.insert("d".to_string(), item("discord_status_icon_1"));
        assert!(attach_badges(&mut items, &entries));
        assert_eq!(items["d"].launcher.as_ref().unwrap().count, Some(5));
        // La conexión se va: la insignia también.
        assert!(forget_sender(&mut entries, ":1.8"));
        assert!(attach_badges(&mut items, &entries));
        assert_eq!(items["d"].launcher, None);
        assert!(!forget_sender(&mut entries, ":1.8"));
    }
}
