use super::Applet;
use crate::structs::MediaInfo;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::json;
use serde_json::Map as JsonMap;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter};
use zbus::{
    blocking::{fdo::DBusProxy, Connection as BlockingConnection, Proxy as BlockingProxy},
    fdo::DBusProxy as AsyncDBusProxy,
    zvariant::Value,
    MessageStream,
    MessageType,
    Connection as AsyncConnection,
    Proxy as AsyncProxy,
};

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

async fn monitor_signals_async(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let conn = AsyncConnection::session().await?;
    let match_rule =
        "type='signal',interface='org.freedesktop.DBus.Properties',member='PropertiesChanged'";
    match conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "AddMatch",
            &(match_rule,),
        )
        .await
    {
        Ok(_) => log::info!("[music] monitor_signals_async: AddMatch succeeded"),
        Err(e) => log::error!("[music] monitor_signals_async: AddMatch failed: {:?}", e),
    }

    let mut stream = MessageStream::from(conn.clone());
    
    // Provide a way to check for pending updates
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

    // Task to handle debounced updates
    let app_handle = app.clone();
    let conn_clone = conn.clone();
    tokio::spawn(async move {
        while let Some(_) = rx.recv().await {
            match fetch_now_playing_async(&conn_clone).await {
                Ok(info) => {
                    let _ = app_handle.emit("music-playing-update", info);
                }
                Err(e) => {
                    let _ = app_handle.emit("mpris-error", e);
                }
            }
        }
    });

    let debounce_duration = std::time::Duration::from_millis(200);
    let mut deadline: Option<tokio::time::Instant> = None;
    let mut dirty = false;

    loop {
        tokio::select! {
            Some(msg_res) = stream.next() => {
                if let Ok(m) = msg_res {
                    let hdr = m.header();
                    if hdr.message_type() == MessageType::Signal {
                         let iface = hdr.interface().map(|i| i.as_str().to_string());
                         let member = hdr.member().map(|me| me.as_str().to_string());
                         
                         if iface.as_deref() == Some("org.freedesktop.DBus.Properties") 
                            && member.as_deref() == Some("PropertiesChanged") 
                         {
                             // Efficiently deserialize body
                             // Signature: sa{sv}as (interface_name, changed_properties, invalidated_properties)
                             let body = m.body();
                             if let Ok((iface_name, changed_props, _invalidated)) = body.deserialize::<(String, HashMap<String, Value>, Vec<String>)>() {
                                 if iface_name == "org.mpris.MediaPlayer2.Player" {
                                     // Check if relevant keys changed
                                    let keys = ["Metadata", "PlaybackStatus"];
                                    let relevant = changed_props.keys().any(|k| keys.contains(&k.as_str()));
                                    if relevant {
                                        dirty = true;
                                        deadline = Some(tokio::time::Instant::now() + debounce_duration);
                                    }
                                 }
                             }
                         }
                    }
                }
            }
            _ = async { match deadline { Some(d) => tokio::time::sleep_until(d).await, None => std::future::pending().await } }, if deadline.is_some() => {
                if dirty {
                    if let Err(e) = tx.send(()).await {
                        eprintln!("Failed to trigger update: {}", e);
                        break;
                    }
                    dirty = false;
                    deadline = None;
                }
            }
        }
    }
    Ok(())
}

async fn fetch_now_playing_async(conn: &AsyncConnection) -> Result<serde_json::Value, String> {
    let dbus = AsyncDBusProxy::new(conn).await.map_err(|e| format!("DBus proxy creation failed: {}", e))?;
    let owned_names = dbus
        .list_names()
        .await
        .map_err(|e| format!("ListNames failed: {}", e))?;
    let names: Vec<String> = owned_names.into_iter().map(|n| n.to_string()).collect();

    // Buscar reproductor que esté reproduciendo; si no, tomar el primero con metadata.
    let mut fallback: Option<(String, Option<String>)> = None; // (service, status)
    for name in names
        .into_iter()
        .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
    {
        let proxy = AsyncProxy::new(
            conn,
            name.as_str(),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        ).await.map_err(|e| format!("Proxy creation failed: {}", e))?;
        
        // PlaybackStatus
        let status: Option<String> = proxy.get_property("PlaybackStatus").await.ok();
        // Preferir Playing
        if status.as_deref() == Some("Playing") {
            let (title, artist, art_url) =
                match proxy.get_property::<HashMap<String, Value>>("Metadata").await.ok() {
                    Some(meta_map) => {
                        // Serializar entrada por entrada y normalizar wrappers
                        let mut jm: JsonMap<String, JsonValue> = JsonMap::new();
                        for (k, v) in meta_map.into_iter() {
                            if let Ok(jv) = serde_json::to_value(&v) {
                                jm.insert(k, normalize_json_value(jv));
                            }
                        }
                        let jobj = JsonValue::Object(jm);
                        extract_mpris_from_json(&jobj)
                    }
                    _none => (None, None, None),
                };
            return Ok(json!({
                "title": title.unwrap_or_else(|| "Unknown".to_string()),
                "artist": artist.unwrap_or_else(|| "Unknown".to_string()),
                "artUrl": art_url.unwrap_or_else(|| "".to_string()),
                "player": name,
                "status": status,
            }));
        } else if fallback.is_none() {
            fallback = Some((name.clone(), status));
        }
    }

    // Si no hay ninguno Playing, intentar fallback
    if let Some((name, status)) = fallback {
        let proxy = AsyncProxy::new(
            conn,
            name.as_str(),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        ).await.map_err(|e| format!("Proxy for fallback failed: {}", e))?;

        let (title, artist, art_url) = match proxy
            .get_property::<HashMap<String, Value>>("Metadata")
            .await
            .ok()
        {
            Some(meta_map) => {
                let mut jm: JsonMap<String, JsonValue> = JsonMap::new();
                for (k, v) in meta_map.into_iter() {
                    if let Ok(jv) = serde_json::to_value(&v) {
                        jm.insert(k, normalize_json_value(jv));
                    }
                }
                let jobj = JsonValue::Object(jm);
                extract_mpris_from_json(&jobj)
            }
            _none => (None, None, None),
        };
        return Ok(json!({
            "title": title.unwrap_or_else(|| "Unknown".to_string()),
            "artist": artist.unwrap_or_else(|| "Unknown".to_string()),
            "artUrl": art_url.unwrap_or_else(|| "".to_string()),
            "player": name,
            "status": status,
        }));
    }

    // Ningún reproductor MPRIS encontrado
    Ok(json!({ "title": "Nothing playing" }))
}

pub fn fetch_now_playing() -> Result<serde_json::Value, String> {
    let conn = BlockingConnection::session().map_err(|e| format!("zbus connect error: {}", e))?;
    let dbus = DBusProxy::new(&conn).map_err(|e| format!("DBus proxy creation failed: {}", e))?;
    let owned_names = dbus
        .list_names()
        .map_err(|e| format!("ListNames failed: {}", e))?;
    let names: Vec<String> = owned_names.into_iter().map(|n| n.to_string()).collect();

    // Buscar reproductor que esté reproduciendo; si no, tomar el primero con metadata.
    let mut fallback: Option<(String, Option<String>)> = None; // (service, status)
    for name in names
        .into_iter()
        .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
    {
        let proxy = BlockingProxy::new(
            &conn,
            name.as_str(),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        );
        if let Ok(p) = proxy {
            // PlaybackStatus
            let status: Option<String> = p.get_property("PlaybackStatus").ok();
            // Preferir Playing
            if status.as_deref() == Some("Playing") {
                let (title, artist, art_url) =
                    match p.get_property::<HashMap<String, Value>>("Metadata").ok() {
                        Some(meta_map) => {
                            // Serializar entrada por entrada y normalizar wrappers
                            let mut jm: JsonMap<String, JsonValue> = JsonMap::new();
                            for (k, v) in meta_map.into_iter() {
                                if let Ok(jv) = serde_json::to_value(&v) {
                                    jm.insert(k, normalize_json_value(jv));
                                }
                            }
                            let jobj = JsonValue::Object(jm);
                            extract_mpris_from_json(&jobj)
                        }
                        _none => (None, None, None),
                    };
                return Ok(json!({
                    "title": title.unwrap_or_else(|| "Unknown".to_string()),
                    "artist": artist.unwrap_or_else(|| "Unknown".to_string()),
                    "artUrl": art_url.unwrap_or_else(|| "".to_string()),
                    "player": name,
                    "status": status,
                }));
            } else if fallback.is_none() {
                fallback = Some((name.clone(), status));
            }
        }
    }

    // Si no hay ninguno Playing, intentar fallback
    if let Some((name, status)) = fallback {
        // reconectar proxy y tratar de extraer metadata (si falla, devolver solo player/status)
        let proxy = BlockingProxy::new(
            &conn,
            name.as_str(),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .map_err(|e| format!("Proxy for fallback failed: {}", e))?;
        let (title, artist, art_url) = match proxy
            .get_property::<HashMap<String, Value>>("Metadata")
            .ok()
        {
            Some(meta_map) => {
                let mut jm: JsonMap<String, JsonValue> = JsonMap::new();
                for (k, v) in meta_map.into_iter() {
                    if let Ok(jv) = serde_json::to_value(&v) {
                        jm.insert(k, normalize_json_value(jv));
                    }
                }
                let jobj = JsonValue::Object(jm);
                extract_mpris_from_json(&jobj)
            }
            _none => (None, None, None),
        };
        return Ok(json!({
            "title": title.unwrap_or_else(|| "Unknown".to_string()),
            "artist": artist.unwrap_or_else(|| "Unknown".to_string()),
            "artUrl": art_url.unwrap_or_else(|| "".to_string()),
            "player": name,
            "status": status,
        }));
    }

    // Ningún reproductor MPRIS encontrado
    Ok(json!({ "title": "Nothing playing" }))
}

pub fn mpris_playpause(player: String) -> Result<(), String> {
    call_player_method(&player, "PlayPause")
}

pub fn mpris_next(player: String) -> Result<(), String> {
    call_player_method(&player, "Next")
}

pub fn mpris_previous(player: String) -> Result<(), String> {
    call_player_method(&player, "Previous")
}

fn call_player_method(player: &str, method: &str) -> Result<(), String> {
    let conn = BlockingConnection::session().map_err(|e| format!("zbus connect error: {}", e))?;
    let proxy = BlockingProxy::new(
        &conn,
        player,
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2.Player",
    )
    .map_err(|e| format!("Proxy creation failed for {}: {}", player, e))?;
    proxy
        .call_method(method, &())
        .map_err(|e| format!("Call {} failed: {}", method, e))?;
    Ok(())
}

fn extract_mpris_from_json(j: &JsonValue) -> (Option<String>, Option<String>, Option<String>) {
    fn find_by_key(j: &JsonValue, key_sub: &str) -> Option<JsonValue> {
        if let Some(obj) = j.as_object() {
            for (k, v) in obj {
                if k.to_lowercase().contains(&key_sub.to_lowercase()) {
                    return Some(v.clone());
                }
            }
        }
        None
    }

    fn as_string_or_first_array(v: &JsonValue) -> Option<String> {
        if v.is_string() {
            v.as_str().map(|s| s.to_string())
        } else if v.is_array() {
            v.as_array()
                .and_then(|arr| arr.iter().find_map(|e| e.as_str().map(|s| s.to_string())))
        } else if v.is_object() {
            v.as_object().and_then(|map| {
                map.values()
                    .find_map(|val| val.as_str().map(|s| s.to_string()))
            })
        } else {
            None
        }
    }

    let title = find_by_key(j, "title")
        .and_then(|v| as_string_or_first_array(&v))
        .or_else(|| {
            if let Some(obj) = j.as_object() {
                for (_k, v) in obj {
                    if let Some(s) = as_string_or_first_array(v) {
                        return Some(s);
                    }
                }
            }
            None
        });

    let artist = find_by_key(j, "artist").and_then(|v| {
        if v.is_array() {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|e| e.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
                    .join("; ")
            })
        } else {
            as_string_or_first_array(&v)
        }
    });

    let art_candidate = find_by_key(j, "art")
        .or_else(|| find_by_key(j, "mpris:arturl"))
        .or_else(|| find_by_key(j, "albumart"))
        .or_else(|| {
            if let Some(obj) = j.as_object() {
                for (_k, v) in obj {
                    if let Some(s) = v.as_str() {
                        let sl = s.to_lowercase();
                        if sl.contains("http")
                            || sl.contains("file://")
                            || sl.ends_with(".png")
                            || sl.ends_with(".jpg")
                            || sl.ends_with(".jpeg")
                        {
                            return Some(JsonValue::String(s.to_string()));
                        }
                    } else if v.is_array() {
                        if let Some(arr) = v.as_array() {
                            for it in arr {
                                if let Some(s) = it.as_str() {
                                    let sl = s.to_lowercase();
                                    if sl.contains("http")
                                        || sl.contains("file://")
                                        || sl.ends_with(".png")
                                        || sl.ends_with(".jpg")
                                        || sl.ends_with(".jpeg")
                                    {
                                        return Some(JsonValue::String(s.to_string()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            None
        });
    let arturl = art_candidate.and_then(|v| v.as_str().map(|s| s.to_string()));

    (title, artist, arturl)
}

fn normalize_json_value(mut v: JsonValue) -> JsonValue {
    loop {
        match v {
            JsonValue::Object(map) if map.len() == 1 => {
                let (_, inner) = map.into_iter().next().map(|(k, v)| (k, v)).unwrap_or(("".to_string(), JsonValue::Null));
                v = inner;
                continue;
            }
            JsonValue::Object(mut map) => {
                if let Some(inner) = map.remove("zvariant::Value::Value") {
                    v = inner;
                    continue;
                }
                let mut out_map: JsonMap<String, JsonValue> = JsonMap::new();
                for (k, val) in map.into_iter() {
                    out_map.insert(k, normalize_json_value(val));
                }
                return JsonValue::Object(out_map);
            }
            JsonValue::Array(arr) => {
                let new_arr: Vec<JsonValue> =
                    arr.into_iter().map(|e| normalize_json_value(e)).collect();
                return JsonValue::Array(new_arr);
            }
            _ => return v,
        }
    }
}

// Implement media info default
impl Default for MediaInfo {
    fn default() -> Self {
        Self {
            title: None,
            artist: None,
            album_art_url: None,
            player: None,
            status: None,
        }
    }
}
