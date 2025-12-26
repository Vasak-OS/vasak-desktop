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
use once_cell::sync::Lazy;
use zbus::{
    blocking::{Connection as BlockingConnection, Proxy as BlockingProxy},
    fdo::DBusProxy as AsyncDBusProxy,
    zvariant::Value,
    MessageStream, MessageType, Connection as AsyncConnection, Proxy as AsyncProxy,
};

// Global state for Active Player to ensure commands use the correct target
// Using a simple Mutex for thread safety across the async monitor and sync commands
static ACTIVE_PLAYER: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

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
    let conn = AsyncConnection::session().await?;
    
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
         update_ui(&app, &init_info);
    }

    loop {
        tokio::select! {
            // 1. POLLING FALLBACK
            _ = poll_interval.tick() => {
                 let current = get_active_player();
                 if let Some(player) = current {
                      // Verify current player is still alive
                      if let Ok(info) = fetch_player_info(&conn, &player).await {
                           update_ui(&app, &info);
                      } else {
                           // Player died, reset
                           set_active_player(None);
                           if let Ok(fallback) = fetch_best_player(&conn).await {
                                update_ui(&app, &fallback);
                           }
                      }
                 } else {
                      // No active player, try to find one
                      if let Ok(fallback) = fetch_best_player(&conn).await {
                           update_ui(&app, &fallback);
                      }
                 }
            }

            // 2. SIGNAL STREAM
            Some(msg_res) = stream.next() => {
                if let Ok(m) = msg_res {
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
            }

            // 3. DEBOUNCE DEADLINE
            _ = async { match deadline { Some(d) => tokio::time::sleep_until(d).await, None => std::future::pending().await } }, if deadline.is_some() => {
                 if let Some(sender) = pending_sender.take() {
                      handle_player_event(&app, &conn, sender).await;
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
    let identity = info.get("player_identity").and_then(|s| s.as_str()).unwrap_or("").to_lowercase();
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

    // 3b. Filter allowed players (browsers/media apps). Ignore unknown senders unless they're current.
    if !is_current && !is_allowed_player(&identity, &target_player) {
        log::info!(
            "[music] Ignoring sender (not allowed): sender={} identity={} status={} current={:?} target={}",
            sender, identity, status, current, target_player
        );
        return;
    }

    // Ignore events from ourselves (defensive)
    if identity.contains("vasak") || sender.contains("vasak") {
        return;
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

fn is_allowed_player(identity: &str, bus: &str) -> bool {
    // Whitelist common media apps and browsers; block random session buses like :1.73
    const MEDIA_APPS: [&str; 14] = [
        "spotify", "vlc", "mpv", "rhythmbox", "clementine", "audacious", "mixxx", "deezer",
        "tidal", "foobar", "amarok", "lollypop", "pragha", "plex"
    ];
    const BROWSERS: [&str; 8] = ["chrome", "chromium", "brave", "firefox", "opera", "edge", "vivaldi", "safari"];

    let id = identity;
    if MEDIA_APPS.iter().any(|k| id.contains(k)) { return true; }
    if BROWSERS.iter().any(|k| id.contains(k)) { return true; }

    // Allow explicit MPRIS well-known names for browsers even if identity is missing
    let bus_lower = bus.to_lowercase();
    if bus_lower.starts_with("org.mpris.mediaplayer2.chrome")
        || bus_lower.starts_with("org.mpris.mediaplayer2.chromium")
        || bus_lower.starts_with("org.mpris.mediaplayer2.brave")
        || bus_lower.starts_with("org.mpris.mediaplayer2.firefox")
        || bus_lower.starts_with("org.mpris.mediaplayer2.vivaldi")
    {
        return true;
    }

    false
}

fn is_well_known_bus(bus: &str) -> bool {
    bus.starts_with("org.mpris.MediaPlayer2.")
}

// --- STATE HELPERS ---

fn get_active_player() -> Option<String> {
    ACTIVE_PLAYER.lock().unwrap().clone()
}

fn set_active_player(p: Option<String>) {
    let mut lock = ACTIVE_PLAYER.lock().unwrap();
    *lock = p;
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

async fn fetch_player_info(conn: &AsyncConnection, name: &str) -> Result<serde_json::Value, String> {
    let proxy = AsyncProxy::new(conn, name, "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player")
        .await
        .map_err(|e| e.to_string())?;

    // Get Status
    let status = proxy.get_property::<String>("PlaybackStatus").await.ok().unwrap_or("Stopped".to_string());
    
    // Get Identity (for browser detection)
    // Try to get "Identity" from root interface
    let root_proxy = AsyncProxy::new(conn, name, "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2")
         .await.ok();
    let identity = if let Some(rp) = root_proxy {
         rp.get_property::<String>("Identity").await.ok().unwrap_or(name.to_string())
    } else {
         name.to_string()
    };

    // Get Metadata
    let (title, artist, art_url) = match proxy.get_property::<HashMap<String, Value>>("Metadata").await {
         Ok(meta) => parse_metadata(meta),
         _ => (None, None, None)
    };

    Ok(json!({
        "player": name, // Unique Name used for commands
        "player_identity": identity, // Human readable / Type detection
        "status": status,
        "title": title.unwrap_or("Unknown".to_string()),
        "artist": artist.unwrap_or("Unknown".to_string()),
        "artUrl": art_url.unwrap_or("".to_string())
    }))
}

fn fetch_player_info_blocking(name: &str) -> Result<serde_json::Value, String> {
    let conn = BlockingConnection::session().map_err(|e| e.to_string())?;
    let proxy = BlockingProxy::new(&conn, name, "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player")
        .map_err(|e| e.to_string())?;

    let status = proxy.get_property::<String>("PlaybackStatus").ok().unwrap_or("Stopped".to_string());

    let root_proxy = BlockingProxy::new(&conn, name, "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2")
        .ok();
    let identity = if let Some(rp) = root_proxy {
        rp.get_property::<String>("Identity").ok().unwrap_or(name.to_string())
    } else {
        name.to_string()
    };

    let (title, artist, art_url) = match proxy.get_property::<HashMap<String, Value>>("Metadata") {
        Ok(meta) => parse_metadata(meta),
        _ => (None, None, None),
    };

    Ok(json!({
        "player": name,
        "player_identity": identity,
        "status": status,
        "title": title.unwrap_or("Unknown".to_string()),
        "artist": artist.unwrap_or("Unknown".to_string()),
        "artUrl": art_url.unwrap_or("".to_string())
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
             let identity = info.get("player_identity").and_then(|s| s.as_str()).unwrap_or("").to_lowercase();
             
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
        Ok(json!({ 
            "title": "Nothing playing", 
            "artist": "", "artUrl": "", "player": "", "status": "Stopped" 
        }))
    }
}

fn parse_metadata(meta: HashMap<String, Value>) -> (Option<String>, Option<String>, Option<String>) {
    let mut jm = JsonMap::new();
    for (k, v) in meta {
        if let Ok(jv) = serde_json::to_value(&v) {
            jm.insert(k, normalize_json(jv));
        }
    }
    let jobj = JsonValue::Object(jm);
    
    let title = find_str(&jobj, &["xesam:title", "title"]);
    let artist = find_str_array(&jobj, &["xesam:artist", "artist"]);
    let art = find_str(&jobj, &["mpris:artUrl", "artUrl", "albumArt"]);

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

fn normalize_json(v: JsonValue) -> JsonValue {
    match v {
        JsonValue::Object(mut map) => {
             if map.len() == 1 {
                 if let Some(inner) = map.remove("zvariant::Value::Value") {
                     return normalize_json(inner);
                 }
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

pub fn fetch_now_playing() -> Result<serde_json::Value, String> {
    // Return what the Monitor thinks is active
    let active = get_active_player();
    if let Some(player) = active {
         return fetch_player_info_blocking(&player);
    }
    Ok(json!({ "title": "Nothing playing", "status": "Stopped" }))
}

pub fn mpris_playpause(player: String) -> Result<String, String> {
    let target = resolve_target(player);
    // read current status
    let status = BlockingConnection::session()
        .map_err(|e| e.to_string())
        .and_then(|conn| {
            BlockingProxy::new(&conn, target.as_str(), "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player")
                .map_err(|e| e.to_string())
                .and_then(|proxy| Ok(proxy.get_property::<String>("PlaybackStatus").ok().unwrap_or("Unknown".to_string())))
        }).unwrap_or_else(|_| "Unknown".to_string());
    let method = if status == "Paused" { "Play" } else if status == "Playing" { "Pause" } else { "PlayPause" };
    exec_command(&target, method)?;
    Ok(target)
}

pub fn mpris_next(player: String) -> Result<String, String> {
    let target = resolve_target(player);
    exec_command(&target, "Next")?;
    Ok(target)
}

pub fn mpris_previous(player: String) -> Result<String, String> {
    let target = resolve_target(player);
    exec_command(&target, "Previous")?;
    Ok(target)
}

fn resolve_target(inc: String) -> String {
    if !inc.is_empty() {
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

fn exec_command(player: &str, method: &str) -> Result<(), String> {
    if player.is_empty() { return Err("No player selected".to_string()); }
    
    let conn = BlockingConnection::session().map_err(|e| e.to_string())?;
    let proxy = BlockingProxy::new(&conn, player, "/org/mpris/MediaPlayer2", "org.mpris.MediaPlayer2.Player")
        .map_err(|e| format!("Proxy creation failed: {}", e))?;
    proxy.call_method(method, &()).map_err(|e| format!("Method call failed: {}", e))?;
    Ok(())
}

pub fn emit_now_playing(app: &AppHandle, player: &str) -> Result<(), String> {
    if player.is_empty() { return Err("No player selected".to_string()); }
    log::info!("[music] emit_now_playing player={}", player);
    let info = match fetch_player_info_blocking(player) {
        Ok(i) => i,
        Err(e) => {
            log::warn!("[music] emit_now_playing fetch failed: {}", e);
            // Return minimal info so UI updates even if fetch fails
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
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        if let Ok(fresh_info) = fetch_player_info_blocking(&player_clone) {
            let _ = app_clone.emit("music-playing-update", &fresh_info);
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
