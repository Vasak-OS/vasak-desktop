#![allow(clippy::if_same_then_else)]
#![allow(clippy::derivable_impls)]
use super::Applet;
use crate::structs::MediaInfo;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::json;
use serde_json::Map as JsonMap;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use std::sync::LazyLock;
use zbus::{
    fdo::DBusProxy as AsyncDBusProxy,
    zvariant::{ObjectPath, Value},
    MessageStream, MessageType, Connection as AsyncConnection, Proxy as AsyncProxy,
};

// Global state for Active Player to ensure commands use the correct target
// Using a simple Mutex for thread safety across the async monitor and sync commands
static ACTIVE_PLAYER: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

/// El que eligió el usuario, que no es lo mismo que el que está activo.
///
/// `ACTIVE_PLAYER` lo mueve la heurística cada vez que algo empieza a sonar.
/// Éste sólo lo mueve una elección, y mientras esté puesto la heurística no
/// cambia de foco. Se suelta cuando ese reproductor desaparece del bus.
static PINNED_PLAYER: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

pub struct MusicApplet;

#[async_trait]
impl Applet for MusicApplet {
    fn name(&self) -> &'static str {
        "music"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        monitor_signals_async(app).await
    }
}

// --- LOGIC: Event Monitoring & State Management ---

async fn monitor_signals_async(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let mut reconnect_attempts = 0u32;
    let max_reconnects = 5;
    
    loop {
        match monitor_with_reconnect(&app, reconnect_attempts).await {
            Ok(_) => {
                log::info!("[music] Monitor loop ended normally");
                break;
            }
            Err(e) => {
                reconnect_attempts += 1;
                if reconnect_attempts >= max_reconnects {
                    log::error!("[music] Max reconnection attempts reached: {}", e);
                    let _ = app.emit("dbus-status", serde_json::json!({
                        "service": "music",
                        "status": "failed",
                        "message": "No se pudo conectar al bus de sesión"
                    }));
                    break;
                }
                log::warn!("[music] Connection lost (attempt {}): {}. Reconnecting...", reconnect_attempts, e);
                let _ = app.emit("dbus-status", serde_json::json!({
                    "service": "music",
                    "status": "reconnecting",
                    "attempt": reconnect_attempts
                }));
                tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(reconnect_attempts.min(3)))).await;
            }
        }
    }
    Ok(())
}

async fn monitor_with_reconnect(app: &AppHandle, attempt: u32) -> Result<(), String> {
    let conn = AsyncConnection::session().await.map_err(|e| e.to_string())?;
    
    if attempt > 0 {
        log::info!("[music] Reconnected successfully after {} attempts", attempt);
        let _ = app.emit("dbus-status", serde_json::json!({
            "service": "music",
            "status": "connected"
        }));
    }
    
    // Subscribe to PropertiesChanged
    let match_rule = "type='signal',interface='org.freedesktop.DBus.Properties',member='PropertiesChanged'";
    let _ = conn.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "AddMatch",
        &(match_rule,),
    ).await;

    log::info!("[music] Started Signal Monitor");

    let mut stream = MessageStream::from(conn.clone());
    let debounce_duration = std::time::Duration::from_millis(150);
    let mut deadline: Option<tokio::time::Instant> = None;
    let mut pending_sender: Option<String> = None;
    
    // Polling Interval for redundancy
    let mut poll_interval = tokio::time::interval(std::time::Duration::from_secs(2));

    // Initialize state
    if let Ok(init_info) = fetch_best_player(&conn).await {
         update_ui(app, &init_info);
    }

    loop {
        tokio::select! {
            // 1. POLLING FALLBACK
            _ = poll_interval.tick() => {
                 // El elegido a mano manda sobre el que puso la heurística.
                 let current = get_pinned_player().or_else(get_active_player);
                 if let Some(player) = current {
                      // Verify current player is still alive
                      if let Ok(info) = fetch_player_info(&conn, &player).await {
                           update_ui(app, &info);
                      } else {
                           // Player died, reset
                           set_active_player(None);
                           // La elección a mano sólo se suelta si el reproductor
                           // **se fue del bus**. Un `GetAll` que no contestó en
                           // 500 ms no es una muerte: un navegador ocupado pasa
                           // ese plazo, y perder ahí la elección sería mover el
                           // foco a otro reproductor sin que nadie lo pidiera.
                           if !player_available_async(&conn, &player).await {
                                set_pinned_player(None);
                           }
                           if let Ok(fallback) = fetch_best_player(&conn).await {
                                update_ui(app, &fallback);
                           }
                      }
                 } else {
                      // No active player, try to find one
                      if let Ok(fallback) = fetch_best_player(&conn).await {
                           update_ui(app, &fallback);
                      }
                 }
            }

            // 2. SIGNAL STREAM
            Some(msg_res) = stream.next() => {
                match msg_res {
                    Ok(m) => {
                        let hdr = m.header();
                        if hdr.message_type() == MessageType::Signal {
                             if let Some(sender) = hdr.sender().map(|s| s.as_str().to_string()) {
                                 let iface = hdr.interface().map(|i| i.as_str().to_string());
                                 if iface.as_deref() == Some("org.freedesktop.DBus.Properties") {
                                      // Debounce trigger
                                      pending_sender = Some(sender);
                                      deadline = Some(tokio::time::Instant::now() + debounce_duration);
                                 }
                             }
                        }
                    }
                    Err(e) => {
                        log::error!("[music] Stream error: {}", e);
                        return Err(e.to_string());
                    }
                }
            }

            // 3. DEBOUNCE DEADLINE
            _ = async { match deadline { Some(d) => tokio::time::sleep_until(d).await, None => std::future::pending().await } }, if deadline.is_some() => {
                 if let Some(sender) = pending_sender.take() {
                      handle_player_event(app, &conn, sender).await;
                 }
                 deadline = None;
            }
        }
    }
}

// Core Logic: Decides if an event should update the focus
async fn handle_player_event(app: &AppHandle, conn: &AsyncConnection, sender: String) {
    // 1. Fetch Sender Info
    let info = match fetch_player_info(conn, &sender).await {
        Ok(i) => i,
        Err(e) => {
            log::warn!("[music] fetch_player_info failed sender={} err={}", sender, e);
            return; // Ignore ghost events
        }
    };

    let status = info.get("status").and_then(|s| s.as_str()).unwrap_or("Stopped");
    let identity = info.get("playerIdentity").and_then(|s| s.as_str()).unwrap_or("").to_lowercase();
    let is_playing = status == "Playing";

    // 3. Check Current State
    let current = get_active_player();
    let is_ephemeral = sender.starts_with(":");
    let target_player = if is_ephemeral {
        current.clone().unwrap_or(sender.clone())
    } else {
        sender.clone()
    };
    let is_current = current.as_ref() == Some(&target_player);

    // 3b. Un nombre de bus MPRIS de verdad, y no nosotros.
    //
    // Antes acá había una lista de nombres —catorce reproductores y ocho
    // navegadores— y lo que no estuviera escrito en ella se ignoraba. Eso deja
    // afuera a cualquier reproductor que no se le haya ocurrido a nadie:
    // `ncspot`, `mpDris2`, un cliente de mpd, el que salga mañana. Publicar
    // `org.mpris.MediaPlayer2.*` **es** el contrato; alcanza con exigir eso.
    if !is_current && !is_well_known_bus(&target_player) {
        log::info!(
            "[music] Ignoring sender (not an MPRIS bus): sender={} status={} current={:?} target={}",
            sender, status, current, target_player
        );
        return;
    }

    // Ignore events from ourselves (defensive)
    if identity.contains("vasak") || sender.contains("vasak") {
        return;
    }

    // 3c. Si el usuario eligió un reproductor a mano, manda él.
    //
    // La puntuación de `fetch_best_player` es una suposición razonable para
    // arrancar, pero en cuanto alguien elige, seguir cambiando de foco solo es
    // desobedecer. El elegido se suelta cuando muere, no antes.
    if let Some(pinned) = get_pinned_player() {
        if target_player != pinned {
            return;
        }
    }

    // 4. Decision Matrix
    // If the sender is ephemeral and differs from the target, just emit update without switching focus.
    if is_ephemeral && sender != target_player {
        let mut info_mut = info;
        info_mut["player"] = json!(target_player.clone());
        let _ = app.emit("music-playing-update", &info_mut);
        return;
    }

    let should_switch = if is_current {
        true // refresh same player
    } else if is_playing {
        true // only switch focus on actively playing senders
    } else if current.is_none() {
        true // no active yet, take first event
    } else {
        false // ignore paused/stopped events from other players to keep last playing
    };

    if should_switch {
        if is_well_known_bus(&target_player) {
            set_active_player(Some(target_player.clone()));
        }
        let mut info_mut = info;
        if is_ephemeral {
            info_mut["player"] = json!(target_player.clone());
        }
        let _ = app.emit("music-playing-update", &info_mut);
    }
}

fn is_well_known_bus(bus: &str) -> bool {
    bus.starts_with("org.mpris.MediaPlayer2.")
}

fn is_valid_bus_name(bus: &str) -> bool {
    if bus.starts_with(":") {
        return bus.len() >= 4;
    }

    is_well_known_bus(bus)
}

// --- STATE HELPERS ---

/// Get the currently active player name
pub fn get_active_player() -> Option<String> {
    ACTIVE_PLAYER.lock()
        .ok()
        .and_then(|guard| guard.clone())
}

/// Set the active player
pub fn set_active_player(player: Option<String>) {
    if let Ok(mut lock) = ACTIVE_PLAYER.lock() {
        *lock = player;
    } else {
        log::error!("Failed to acquire ACTIVE_PLAYER lock");
    }
}

/// El reproductor que eligió el usuario, si eligió alguno.
pub fn get_pinned_player() -> Option<String> {
    PINNED_PLAYER.lock().ok().and_then(|guard| guard.clone())
}

/// Fija —o suelta, con `None`— el reproductor elegido a mano.
pub fn set_pinned_player(player: Option<String>) {
    if let Ok(mut lock) = PINNED_PLAYER.lock() {
        *lock = player;
    } else {
        log::error!("Failed to acquire PINNED_PLAYER lock");
    }
}

fn update_ui(app: &AppHandle, info: &serde_json::Value) {
    if let Some(p) = info.get("player").and_then(|s| s.as_str()) {
        if get_active_player().is_none() {
             set_active_player(Some(p.to_string()));
        }
    }
    let _ = app.emit("music-playing-update", info);
}

// --- FETCH HELPERS ---

/// Todo lo que el reproductor sabe de sí mismo, en una sola vuelta al bus.
///
/// Antes se pedía propiedad por propiedad, cada una con su tiempo límite: tres
/// idas y vueltas para tres datos, y ahora harían falta catorce. `GetAll` las
/// trae todas juntas.
///
/// Y de paso arregla lo que preguntar de a una hacía mal: un reproductor que no
/// implementa `Shuffle` **contesta con error** —Chromium lo hace— y eso no
/// significa que esté muerto. En un `GetAll` esa propiedad sencillamente no
/// viene, que es la diferencia entre «no tiene» y «no está».
async fn get_all(conn: &AsyncConnection, name: &str, iface: &str) -> Result<JsonValue, String> {
    let proxy = AsyncProxy::new(
        conn,
        name,
        "/org/mpris/MediaPlayer2",
        "org.freedesktop.DBus.Properties",
    )
    .await
    .map_err(|e| e.to_string())?;

    let reply = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        proxy.call_method("GetAll", &(iface,)),
    )
    .await
    .map_err(|_| format!("{name} no contestó a tiempo"))?
    .map_err(|e| e.to_string())?;

    let body = reply.body();
    let props: HashMap<String, Value> = body.deserialize().map_err(|e| e.to_string())?;
    Ok(a_json(props))
}

/// Un `a{sv}` del bus como JSON, ya sin las cáscaras de `Variant`.
fn a_json(props: HashMap<String, Value>) -> JsonValue {
    let mut jm = JsonMap::new();
    for (k, v) in props {
        if let Ok(jv) = serde_json::to_value(&v) {
            jm.insert(k, normalize_json(jv));
        }
    }
    JsonValue::Object(jm)
}

fn find_bool(j: &JsonValue, key: &str) -> Option<bool> {
    match j.get(key)? {
        JsonValue::Bool(b) => Some(*b),
        otro => extract_string_value(otro).and_then(|s| s.parse().ok()),
    }
}

fn find_i64(j: &JsonValue, key: &str) -> Option<i64> {
    match j.get(key)? {
        JsonValue::Number(n) => n.as_i64(),
        otro => extract_string_value(otro).and_then(|s| s.parse().ok()),
    }
}

fn find_f64(j: &JsonValue, key: &str) -> Option<f64> {
    match j.get(key)? {
        JsonValue::Number(n) => n.as_f64(),
        otro => extract_string_value(otro).and_then(|s| s.parse().ok()),
    }
}

async fn fetch_player_info(conn: &AsyncConnection, name: &str) -> Result<JsonValue, String> {
    let player = get_all(conn, name, "org.mpris.MediaPlayer2.Player").await?;
    // La raíz es opcional: lo que trae —cómo se llama el reproductor y si su
    // ventana se puede traer al frente— es para mostrar, no para controlar.
    let root = get_all(conn, name, "org.mpris.MediaPlayer2")
        .await
        .unwrap_or(JsonValue::Null);

    let meta = player.get("Metadata").cloned().unwrap_or(JsonValue::Null);
    let (title, artist, art_url) = parse_metadata(&meta);

    Ok(json!({
        "player": name,
        "playerIdentity": find_str(&root, &["Identity"]).unwrap_or_else(|| name.to_string()),
        "status": find_str(&player, &["PlaybackStatus"]).unwrap_or_else(|| "Stopped".to_string()),
        "title": title.unwrap_or_default(),
        "artist": artist.unwrap_or_default(),
        "album": find_str(&meta, &["xesam:album"]).unwrap_or_default(),
        "artUrl": art_url.unwrap_or_default(),
        // Los dos en microsegundos, como los manda MPRIS. `length` en 0 quiere
        // decir que el reproductor no dijo cuánto dura —una radio en vivo no lo
        // sabe—, y ahí no hay barra que dibujar.
        "length": find_i64(&meta, "mpris:length").unwrap_or(0),
        "position": find_i64(&player, "Position").unwrap_or(0),
        "trackId": find_str(&meta, &["mpris:trackid"]).unwrap_or_default(),
        // Lo que el reproductor dice que se puede hacer. Sin esto los botones se
        // dibujan siempre y la mitad no hace nada: con un vídeo de YouTube,
        // Chromium contesta que no se puede ir ni al anterior ni al siguiente.
        "canGoNext": find_bool(&player, "CanGoNext").unwrap_or(false),
        "canGoPrevious": find_bool(&player, "CanGoPrevious").unwrap_or(false),
        "canPlay": find_bool(&player, "CanPlay").unwrap_or(false),
        "canPause": find_bool(&player, "CanPause").unwrap_or(false),
        "canSeek": find_bool(&player, "CanSeek").unwrap_or(false),
        "canControl": find_bool(&player, "CanControl").unwrap_or(true),
        "canRaise": find_bool(&root, "CanRaise").unwrap_or(false),
        // Estos tres van en nulo cuando el reproductor no los implementa, y ahí
        // el control no se dibuja. `null` no es `false`: uno es «no tiene» y el
        // otro «lo tiene y está apagado».
        "shuffle": find_bool(&player, "Shuffle"),
        "loopStatus": find_str(&player, &["LoopStatus"]),
        "volume": find_f64(&player, "Volume"),
    }))
}

async fn fetch_best_player(conn: &AsyncConnection) -> Result<serde_json::Value, String> {
    let dbus = AsyncDBusProxy::new(conn).await.map_err(|e| e.to_string())?;
    let names = dbus.list_names().await.map_err(|e| e.to_string())?;
    
    let mut best_score = -1;
    let mut best_info = None;

    let preferred_apps = ["spotify", "rhythmbox", "clementine", "audacious", "vlc", "mpv", "mixxx"];

    for name in names.into_iter().filter(|n| n.starts_with("org.mpris.MediaPlayer2.")) {
        if let Ok(info) = fetch_player_info(conn, &name).await {
             let status = info.get("status").and_then(|s| s.as_str()).unwrap_or("Stopped");
             let identity = info.get("playerIdentity").and_then(|s| s.as_str()).unwrap_or("").to_lowercase();
             
             let is_preferred = preferred_apps.iter().any(|app| identity.contains(app));
             let is_playing = status == "Playing";
             let is_paused = status == "Paused";

             let score = if is_playing && is_preferred { 4 }
                         else if is_playing { 3 }
                         else if is_paused && is_preferred { 2 }
                         else if is_paused { 1 }
                         else { 0 };
             
             if score > best_score {
                 best_score = score;
                 best_info = Some(info);
                 // Early exit if max score
                 if score == 4 { break; }
             }
        }
    }

    if let Some(info) = best_info {
        set_active_player(info.get("player").and_then(|s| s.as_str()).map(|s| s.to_string()));
        Ok(info)
    } else {
        // Sin título y sin reproductor: el texto de «no hay nada sonando» lo
        // pone la vista, que es la que sabe en qué idioma está el usuario.
        Ok(json!({
            "player": "",
            "playerIdentity": "",
            "status": "Stopped",
            "title": "",
            "artist": "",
            "album": "",
            "artUrl": "",
            "length": 0,
            "position": 0,
            "trackId": "",
            "canGoNext": false,
            "canGoPrevious": false,
            "canPlay": false,
            "canPause": false,
            "canSeek": false,
            "canControl": false,
            "canRaise": false,
            "shuffle": JsonValue::Null,
            "loopStatus": JsonValue::Null,
            "volume": JsonValue::Null,
        }))
    }
}

fn parse_metadata(meta: &JsonValue) -> (Option<String>, Option<String>, Option<String>) {
    let title = find_str(meta, &["xesam:title", "title"]);
    let artist = find_str_array(meta, &["xesam:artist", "artist"]);
    let art = find_str(meta, &["mpris:artUrl", "artUrl", "albumArt"]);

    (title, artist, art)
}

fn extract_string_value(v: &JsonValue) -> Option<String> {
    match v {
        JsonValue::String(s) => {
            // ignore DBus type markers
            if s == "s" || s == "as" { None } else { Some(s.clone()) }
        },
        JsonValue::Number(n) => Some(n.to_string()),
        JsonValue::Bool(b) => Some(b.to_string()),
        JsonValue::Array(arr) => arr.iter().find_map(extract_string_value),
        JsonValue::Object(map) => {
            // prefer common 'value' field if present
            if let Some(inner) = map.get("value").or_else(|| map.get("Value")).or_else(|| map.get("contents")) {
                if let Some(s) = extract_string_value(inner) { return Some(s); }
            }
            // unwrap single-key wrapper
            if map.len() == 1 {
                if let Some((_, inner)) = map.iter().next() {
                    if let Some(s) = extract_string_value(inner) { return Some(s); }
                }
            }
            // common wrappers
            for k in ["String", "Str", "OwnedStr", "Text", "Value", "Variant", "Basic"] {
                if let Some(inner) = map.get(k) {
                    if let Some(s) = extract_string_value(inner) { return Some(s); }
                }
            }
            // generic search across all values
            for inner in map.values() {
                if let Some(s) = extract_string_value(inner) { return Some(s); }
            }
            None
        }
        _ => None,
    }
}

fn extract_string_array(v: &JsonValue) -> Vec<String> {
    match v {
        JsonValue::Array(arr) => arr.iter().filter_map(extract_string_value).collect(),
        JsonValue::Object(map) => {
            // unwrap single-key wrapper
            if map.len() == 1 {
                if let Some((_, inner)) = map.iter().next() {
                    return extract_string_array(inner);
                }
            }
            // known array wrappers
            for k in ["Array", "Vec", "List"] {
                if let Some(inner) = map.get(k) { return extract_string_array(inner); }
            }
            // collect strings from all values
            let mut out = Vec::new();
            for inner in map.values() {
                match inner {
                    JsonValue::Array(_) | JsonValue::Object(_) => {
                        out.extend(extract_string_array(inner));
                    }
                    _ => {
                        if let Some(s) = extract_string_value(inner) { out.push(s); }
                    }
                }
            }
            // filter out DBus type markers
            out.into_iter().filter(|s| s != "s" && s != "as").collect()
        }
        JsonValue::String(s) => vec![s.clone()],
        _ => Vec::new(),
    }
}

fn find_str(j: &JsonValue, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = j.get(*key) {
            if let Some(s) = extract_string_value(v) { return Some(s); }
        }
    }
    None
}

fn find_str_array(j: &JsonValue, keys: &[&str]) -> Option<String> {
     for key in keys {
        if let Some(v) = j.get(*key) {
             let parts = extract_string_array(v);
             if !parts.is_empty() { return Some(parts.join(", ")); }
             if let Some(s) = extract_string_value(v) { return Some(s); }
        }
    }
    None
}

/// Saca las cáscaras de `Variant` y deja JSON común.
///
/// Un `Variant` se serializa como **la firma al lado del contenido**, o sea dos
/// claves. Hasta ahora eso sólo se desenvolvía cuando venía solo, y funcionaba
/// de casualidad: quien leía el valor rebuscaba entre todas las claves del
/// objeto y descartaba las que parecían firmas —de ahí el filtro de `"s"` y
/// `"as"`—. Con `GetAll` la firma viene **siempre**, y ese rebusque no sabe
/// leer un booleano ni un entero: el estado entero llegaba vacío.
fn normalize_json(v: JsonValue) -> JsonValue {
    match v {
        JsonValue::Object(mut map) => {
             if let Some(inner) = map.remove("zvariant::Value::Value") {
                 return normalize_json(inner);
             }
             if map.len() == 1 {
                 // If generic variant wrapper 1 key
                 if let Some((_, val)) = map.iter().next() {
                      return normalize_json(val.clone());
                 }
             }
             JsonValue::Object(map.into_iter().map(|(k,v)| (k, normalize_json(v))).collect())
        },
        JsonValue::Array(arr) => JsonValue::Array(arr.into_iter().map(normalize_json).collect()),
        _ => v
    }
}

// --- COMMANDS ---

pub async fn fetch_now_playing() -> Result<serde_json::Value, String> {
    let active = get_active_player();
    if let Some(player) = active {
         if is_valid_bus_name(&player) {
             let conn = AsyncConnection::session().await.map_err(|e| e.to_string())?;
             return fetch_player_info(&conn, &player).await;
         }

         log::warn!("[music] Ignoring invalid active player bus: {}", player);
         set_active_player(None);
    }
    Ok(json!({ "title": "", "status": "Stopped", "player": "" }))
}

pub async fn mpris_playpause(player: String) -> Result<String, String> {
    let target = resolve_target(player);
    if target.is_empty() {
        return Err("No player selected".into());
    }
    let conn = AsyncConnection::session().await.map_err(|e| e.to_string())?;
    let proxy = AsyncProxy::new(&conn, target.as_str(), "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player")
        .await
        .map_err(|e| e.to_string())?;
    let status = proxy.get_property::<String>("PlaybackStatus").await
        .unwrap_or_else(|_| "Unknown".to_string());
    let method = if status == "Paused" { "Play" } else if status == "Playing" { "Pause" } else { "PlayPause" };
    exec_command_async(&conn, &target, method).await?;
    Ok(target)
}

pub async fn mpris_next(player: String) -> Result<String, String> {
    let target = resolve_target(player);
    let conn = AsyncConnection::session().await.map_err(|e| e.to_string())?;
    exec_command_async(&conn, &target, "Next").await?;
    Ok(target)
}

pub async fn mpris_previous(player: String) -> Result<String, String> {
    let target = resolve_target(player);
    let conn = AsyncConnection::session().await.map_err(|e| e.to_string())?;
    exec_command_async(&conn, &target, "Previous").await?;
    Ok(target)
}

/// La conexión y el destino, con el reproductor ya comprobado vivo.
///
/// Los ocho comandos hacían los mismos cuatro pasos antes de poder hablar.
async fn connect_to_player(player: String) -> Result<(AsyncConnection, String), String> {
    let target = resolve_target(player);
    if target.is_empty() {
        return Err("No player selected".into());
    }
    let conn = AsyncConnection::session()
        .await
        .map_err(|e| e.to_string())?;
    if !player_available_async(&conn, &target).await {
        return Err(format!("Player not available: {target}"));
    }
    Ok((conn, target))
}

/// Un proxy de la interfaz del reproductor.
async fn player_proxy<'a>(
    conn: &'a AsyncConnection,
    target: &str,
) -> Result<AsyncProxy<'a>, String> {
    AsyncProxy::new(
        conn,
        target.to_string(),
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2.Player",
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn mpris_stop(player: String) -> Result<String, String> {
    let (conn, target) = connect_to_player(player).await?;
    exec_command_async(&conn, &target, "Stop").await?;
    Ok(target)
}

/// Trae la ventana del reproductor al frente.
///
/// Es de la interfaz raíz y no de la del reproductor, y no todos la implementan:
/// `CanRaise` dice si sirve, y el botón no se dibuja cuando dice que no.
pub async fn mpris_raise(player: String) -> Result<String, String> {
    let (conn, target) = connect_to_player(player).await?;
    let proxy = AsyncProxy::new(
        &conn,
        target.clone(),
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2",
    )
    .await
    .map_err(|e| e.to_string())?;
    proxy
        .call_method("Raise", &())
        .await
        .map_err(|e| format!("Raise failed: {e}"))?;
    Ok(target)
}

/// Salta a un punto de la pista.
///
/// `SetPosition` lleva el identificador de la pista adentro a propósito: entre
/// que se lee la posición y se manda el salto, la pista puede haber cambiado, y
/// sin ese identificador el salto caería sobre la que empezó recién. Los
/// reproductores que no publican un identificador válido —que los hay— se
/// atienden con `Seek`, que es relativo y no lo necesita.
pub async fn mpris_set_position(player: String, micros: i64) -> Result<String, String> {
    let (conn, target) = connect_to_player(player).await?;
    let proxy = player_proxy(&conn, &target).await?;

    let wanted = micros.max(0);
    let info = fetch_player_info(&conn, &target).await?;
    let track = info
        .get("trackId")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    if let Ok(path) = ObjectPath::try_from(track) {
        proxy
            .call_method("SetPosition", &(path, wanted))
            .await
            .map_err(|e| format!("SetPosition failed: {e}"))?;
        return Ok(target);
    }

    let current = info.get("position").and_then(|v| v.as_i64()).unwrap_or(0);
    proxy
        .call_method("Seek", &(wanted - current,))
        .await
        .map_err(|e| format!("Seek failed: {e}"))?;
    Ok(target)
}

/// El volumen **del reproductor**, que no es el de la sesión.
pub async fn mpris_set_volume(player: String, volume: f64) -> Result<String, String> {
    let (conn, target) = connect_to_player(player).await?;
    let proxy = player_proxy(&conn, &target).await?;
    proxy
        .set_property("Volume", volume.clamp(0.0, 1.0))
        .await
        .map_err(|e| format!("Volume failed: {e}"))?;
    Ok(target)
}

pub async fn mpris_set_shuffle(player: String, shuffle: bool) -> Result<String, String> {
    let (conn, target) = connect_to_player(player).await?;
    let proxy = player_proxy(&conn, &target).await?;
    proxy
        .set_property("Shuffle", shuffle)
        .await
        .map_err(|e| format!("Shuffle failed: {e}"))?;
    Ok(target)
}

/// Repetición: `None`, `Track` o `Playlist`, y nada más.
///
/// Los tres valores son los de la especificación y van tal cual por el bus. Se
/// comprueban acá porque el que llega es un texto, y mandar cualquier otro deja
/// al reproductor contestando un error que no dice nada.
pub async fn mpris_set_loop(player: String, status: String) -> Result<String, String> {
    if !matches!(status.as_str(), "None" | "Track" | "Playlist") {
        return Err(format!("LoopStatus inválido: {status}"));
    }
    let (conn, target) = connect_to_player(player).await?;
    let proxy = player_proxy(&conn, &target).await?;
    proxy
        .set_property("LoopStatus", status.as_str())
        .await
        .map_err(|e| format!("LoopStatus failed: {e}"))?;
    Ok(target)
}

/// Todos los reproductores que hay ahora mismo, para poder elegir.
///
/// La lista es lo que hace que elegir sea posible: hasta ahora el escritorio
/// seguía a uno y no había manera de saber cuáles eran los otros.
pub async fn list_players() -> Result<JsonValue, String> {
    let conn = AsyncConnection::session()
        .await
        .map_err(|e| e.to_string())?;
    let dbus = AsyncDBusProxy::new(&conn)
        .await
        .map_err(|e| e.to_string())?;
    let names = dbus.list_names().await.map_err(|e| e.to_string())?;

    let active = get_active_player();
    let pinned = get_pinned_player();
    let mut out = Vec::new();

    for name in names
        .into_iter()
        .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
    {
        let bus = name.to_string();
        if bus.to_lowercase().contains("vasak") {
            continue;
        }
        if let Ok(info) = fetch_player_info(&conn, &bus).await {
            out.push(json!({
                "player": bus,
                "identity": info.get("playerIdentity").cloned().unwrap_or_default(),
                "status": info.get("status").cloned().unwrap_or_default(),
                "title": info.get("title").cloned().unwrap_or_default(),
                "active": active.as_deref() == Some(bus.as_str()),
                "pinned": pinned.as_deref() == Some(bus.as_str()),
            }));
        }
    }

    Ok(JsonValue::Array(out))
}

/// Elige un reproductor a mano. Con el vacío se vuelve a la elección automática.
pub async fn pick_player(app: &AppHandle, player: String) -> Result<String, String> {
    if player.is_empty() {
        set_pinned_player(None);
        let conn = AsyncConnection::session()
            .await
            .map_err(|e| e.to_string())?;
        let info = fetch_best_player(&conn).await?;
        update_ui(app, &info);
        return Ok(String::new());
    }

    if !is_well_known_bus(&player) {
        return Err(format!("No es un reproductor MPRIS: {player}"));
    }
    set_pinned_player(Some(player.clone()));
    set_active_player(Some(player.clone()));
    emit_now_playing(app, &player).await?;
    Ok(player)
}

fn resolve_target(inc: String) -> String {
    if !inc.is_empty() {
        if !is_valid_bus_name(&inc) {
            if let Some(active) = get_active_player() {
                if is_valid_bus_name(&active) {
                    return active;
                }
            }
            return String::new();
        }

        // If caller passes an ephemeral bus (":1.x") but we already have a well-known active, prefer active.
        if inc.starts_with(":") {
            if let Some(active) = get_active_player() {
                if is_well_known_bus(&active) {
                    return active;
                }
            }
        }
        return inc;
    }
    get_active_player().unwrap_or_default()
}

async fn exec_command_async(conn: &AsyncConnection, player: &str, method: &str) -> Result<(), String> {
    if player.is_empty() { return Err("No player selected".to_string()); }

    if !player_available_async(conn, player).await {
        return Err(format!("Player not available: {}", player));
    }

    call_with_retry_async(|| async {
        let proxy = AsyncProxy::new(conn, player, "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player")
            .await
            .map_err(|e| format!("Proxy creation failed: {}", e))?;
        proxy
            .call_method(method, &())
            .await
            .map(|_| ())
            .map_err(|e| format!("Method call failed: {}", e))
    }, 3, 50).await
}

async fn player_available_async(conn: &AsyncConnection, name: &str) -> bool {
    if name.starts_with(":") { return true; }
    match AsyncDBusProxy::new(conn).await {
        Ok(dbus) => dbus.list_names().await.map(|list| list.into_iter().any(|n| n == name)).unwrap_or(false),
        Err(_) => false,
    }
}

async fn call_with_retry_async<F, Fut>(mut f: F, attempts: usize, base_delay_ms: u64) -> Result<(), String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let mut last_err: Option<String> = None;
    for i in 0..attempts {
        match f().await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                if i + 1 < attempts {
                    let delay = base_delay_ms * (1 << i);
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| "Unknown error".to_string()))
}

pub async fn emit_now_playing(app: &AppHandle, player: &str) -> Result<(), String> {
    if player.is_empty() { return Err("No player selected".to_string()); }
    log::info!("[music] emit_now_playing player={}", player);
    let conn = AsyncConnection::session().await.map_err(|e| e.to_string())?;
    let info = match fetch_player_info(&conn, player).await {
        Ok(i) => i,
        Err(e) => {
            log::warn!("[music] emit_now_playing fetch failed: {}", e);
            json!({
                "player": player,
                "status": "Unknown",
                "title": "Loading...",
                "artist": "",
                "artUrl": ""
            })
        }
    };
    if is_well_known_bus(player) {
        set_active_player(Some(player.to_string()));
    }
    update_ui(app, &info);

    // Schedule a delayed refetch to capture state changes post-command
    let app_clone = app.clone();
    let player_clone = player.to_string();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        if let Ok(conn) = AsyncConnection::session().await {
            if let Ok(fresh_info) = fetch_player_info(&conn, &player_clone).await {
                let _ = app_clone.emit("music-playing-update", &fresh_info);
            }
        }
    });

    Ok(())
}

// Legacy impl for traits
impl Default for MediaInfo {
    fn default() -> Self {
        Self { title: None, artist: None, album_art_url: None, player: None, status: None }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// Las propiedades tal como las entrega `GetAll`: un `a{sv}`, con el
    /// `Metadata` adentro como otro diccionario envuelto en su variante.
    fn propiedades_de_getall() -> HashMap<String, Value<'static>> {
        let mut meta: HashMap<String, Value> = HashMap::new();
        meta.insert("xesam:title".into(), Value::from("La pista"));
        meta.insert("xesam:album".into(), Value::from("El disco"));
        meta.insert("mpris:length".into(), Value::from(2_179_701_000i64));
        meta.insert(
            "mpris:artUrl".into(),
            Value::from("file:///tmp/la-tapa.png"),
        );
        meta.insert(
            "xesam:artist".into(),
            Value::from(vec!["Quien la canta".to_string()]),
        );

        let mut props: HashMap<String, Value> = HashMap::new();
        props.insert("PlaybackStatus".into(), Value::from("Playing"));
        props.insert("CanSeek".into(), Value::from(true));
        props.insert("Position".into(), Value::from(634_000_000i64));
        props.insert("Volume".into(), Value::from(0.8f64));
        props.insert("Metadata".into(), Value::from(zvariant::Dict::from(meta)));
        props
    }

    /// Lo que de verdad importa de cambiar trece llamadas por un `GetAll`: que
    /// lo que llega adentro se siga leyendo igual.
    #[test]
    fn los_datos_de_la_pista_salen_del_getall() {
        let props = a_json(propiedades_de_getall());
        let meta = props.get("Metadata").cloned().unwrap_or(JsonValue::Null);

        let (title, artist, art) = parse_metadata(&meta);

        assert_eq!(title.as_deref(), Some("La pista"), "el título");
        assert_eq!(artist.as_deref(), Some("Quien la canta"), "el artista");
        assert_eq!(
            art.as_deref(),
            Some("file:///tmp/la-tapa.png"),
            "la carátula"
        );
        assert_eq!(
            find_str(&meta, &["xesam:album"]).as_deref(),
            Some("El disco")
        );
        assert_eq!(find_i64(&meta, "mpris:length"), Some(2_179_701_000));
    }

    #[test]
    fn el_estado_del_reproductor_tambien() {
        let props = a_json(propiedades_de_getall());

        assert_eq!(
            find_str(&props, &["PlaybackStatus"]).as_deref(),
            Some("Playing")
        );
        assert_eq!(find_bool(&props, "CanSeek"), Some(true));
        assert_eq!(find_i64(&props, "Position"), Some(634_000_000));
        assert_eq!(find_f64(&props, "Volume"), Some(0.8));
    }
}
