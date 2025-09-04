use futures_util::StreamExt; // <- necesario para .next().await en MessageStream
use serde_json::json;
use serde_json::Map as JsonMap;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::thread;
use tauri::{AppHandle, Emitter};
use tokio::runtime::Builder as TokioBuilder;
use zbus::{
    blocking::{fdo::DBusProxy, Connection, Proxy},
    fdo::DBusProxy as AsyncDBusProxy, // <- versión async usada para resolución de owners
    zvariant::Value,
    MessageStream,
    MessageType,
};

use crate::structs::MediaInfo;

// Intenta extraer el vector de bytes decimales desde el debug del Body ("bytes: [ ... ]").
fn parse_bytes_from_debug(body_debug: &str) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::new();
    if let Some(start) = body_debug.find("bytes: [") {
        if let Some(end) = body_debug[start..].find(']') {
            let slice = &body_debug[start + "bytes: [".len()..start + end];
            for token in slice.split(',') {
                let t = token.trim();
                if t.is_empty() {
                    continue;
                }
                // token puede ser un número o texto; intentar parsear decimal
                if let Ok(n) = t.parse::<u8>() {
                    bytes.push(n);
                }
            }
        }
    }
    bytes
}

// Busca claves y extrae la cadena imprimible inmediatamente después (hasta null o límite)
fn extract_metadata_from_bytes(bytes: &[u8], key: &str, max_len: usize) -> Option<String> {
    let key_bytes = key.as_bytes();
    if key_bytes.is_empty() {
        return None;
    }
    for i in 0..bytes.len().saturating_sub(key_bytes.len()) {
        if &bytes[i..i + key_bytes.len()] == key_bytes {
            // avanzar tras la key y posibles delimitadores; buscar el primer byte imprimible
            let mut j = i + key_bytes.len();
            // saltar bytes nulos o pequeños hasta encontrar datos
            while j < bytes.len() && (bytes[j] == 0 || bytes[j] < 0x20) {
                j += 1;
            }
            let mut out: Vec<u8> = Vec::new();
            let mut k = 0usize;
            while j < bytes.len() && k < max_len {
                let b = bytes[j];
                // detener si terminador
                if b == 0 {
                    break;
                }
                if b >= 0x20 && b <= 0x7e {
                    out.push(b);
                } else {
                    // si encontramos no imprimible, terminar
                    break;
                }
                j += 1;
                k += 1;
            }
            if !out.is_empty() {
                if let Ok(s) = String::from_utf8(out) {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn extract_dbus_string_after_signature(bytes: &[u8], key: &str, max_len: usize) -> Option<String> {
    let key_b = key.as_bytes();
    // buscar la key en el stream
    if let Some(pos) = bytes.windows(key_b.len()).position(|w| w == key_b) {
        // buscar 's' (115) en un rango razonable tras la key
        let search_end = std::cmp::min(bytes.len(), pos + key_b.len() + 200);
        let mut i = pos + key_b.len();
        while i < search_end {
            if bytes[i] == b's' {
                // avanzar y saltar bytes de padding / nulos
                let mut k = i + 1;
                while k < bytes.len() && bytes[k] < 0x20 {
                    k += 1;
                }
                // necesitamos 4 bytes de longitud
                if k + 4 <= bytes.len() {
                    let len_slice: [u8; 4] = bytes[k..k + 4].try_into().unwrap_or([0, 0, 0, 0]);
                    let s_len = u32::from_le_bytes(len_slice) as usize;
                    let start = k + 4;
                    let end = start.saturating_add(s_len);
                    if end <= bytes.len() && s_len > 0 {
                        let take = std::cmp::min(s_len, max_len);
                        if let Ok(s) = String::from_utf8(bytes[start..start + take].to_vec()) {
                            return Some(s);
                        }
                    }
                }
            }
            i += 1;
        }
    }
    None
}

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

pub fn fetch_now_playing() -> Result<serde_json::Value, String> {
    let conn = Connection::session().map_err(|e| format!("zbus connect error: {}", e))?;
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
        let proxy = Proxy::new( 
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
        let proxy = Proxy::new(
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


pub fn start_mpris_monitor(app: AppHandle) {
    let app_for_thread = app.clone();
    thread::spawn(move || {
        let rt = TokioBuilder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let _ = rt.block_on(async move {
            if let Err(e) = monitor_signals_async(app_for_thread.clone()).await {
                let _ = app_for_thread.emit("mpris-error", format!("monitor error: {}", e));
            }
        });
    });
}

async fn monitor_signals_async(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let conn = zbus::Connection::session().await?;
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
        Ok(_) => eprintln!("[music] monitor_signals_async: AddMatch succeeded"),
        Err(e) => eprintln!("[music] monitor_signals_async: AddMatch failed: {:?}", e),
    }

    let dbus_async = AsyncDBusProxy::new(&conn).await?;

    // Stream de mensajes del bus
    let stream_conn = conn.clone();
    let mut stream = MessageStream::from(stream_conn);
    while let Some(msg_res) = stream.next().await {
        if let Ok(m) = msg_res {
            let hdr = m.header();
            if hdr.message_type() != MessageType::Signal {
                continue;
            }

            let body = m.body();
            let body_debug = format!("{:#?}", body);

            let keys = [
                "xesam:title",
                "xesam:artist",
                "mpris:artUrl",
                "mpris:arturl",
                "xesam:album",
            ];
            let bytes = parse_bytes_from_debug(&body_debug);
            if !bytes.is_empty() {
                let mut kv: Vec<String> = Vec::new();
                for key in &keys {
                    if let Some(val) = extract_metadata_from_bytes(&bytes, key, 256) {
                        kv.push(format!("{} -> {}", key, val));
                        continue;
                    }
                    if let Some(val) = extract_dbus_string_after_signature(&bytes, key, 1024) {
                        kv.push(format!("{} -> {}", key, val));
                        continue;
                    }
                }
            }

            let iface = hdr.interface().map(|i| i.as_str().to_string());
            let member = hdr.member().map(|me| me.as_str().to_string());
            let sender_opt = hdr.sender().map(|s| s.to_string());

            let mut should_requery = false;
            if iface.as_deref() == Some("org.freedesktop.DBus.Properties")
                && member.as_deref() == Some("PropertiesChanged")
            {
                if let Some(sender_str) = sender_opt {
                    if sender_str.starts_with("org.mpris.MediaPlayer2.") {
                        should_requery = true;
                    } else if sender_str.starts_with(':') {
                        if let Ok(names) = dbus_async.list_names().await {
                            for owned in names
                                .into_iter()
                                .filter(|n| n.as_str().starts_with("org.mpris.MediaPlayer2."))
                            {
                                let bus_name = owned.clone().into();
                                if let Ok(owner) = dbus_async.get_name_owner(bus_name).await {
                                    if owner.to_string() == sender_str {
                                        should_requery = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if should_requery {
                let info_res = tokio::task::spawn_blocking(|| fetch_now_playing()).await;
                if let Ok(Ok(info)) = info_res {
                    let _ = app.emit("music-playing-update", info);
                } else if let Ok(Err(e)) = info_res {
                    let _ = app.emit("mpris-error", e);
                } else if let Err(join_err) = info_res {
                    let _ = app.emit("mpris-error", format!("join error: {:?}", join_err));
                }
            }
        }
    }
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
            // buscar cualquier string que contenga "http" o "file://"
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
                        for it in v.as_array().unwrap() {
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
            None
        });
    let arturl = art_candidate.and_then(|v| v.as_str().map(|s| s.to_string()));

    (title, artist, arturl)
}

fn normalize_json_value(mut v: JsonValue) -> JsonValue {
    loop {
        match v {
            JsonValue::Object(map) if map.len() == 1 => {
                let (_, inner) = map.into_iter().next().unwrap();
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

fn call_player_method(player: &str, method: &str) -> Result<(), String> {
    let conn = Connection::session().map_err(|e| format!("zbus connect error: {}", e))?;
    let proxy = Proxy::new(
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


pub fn mpris_playpause(player: String) -> Result<(), String> {
    call_player_method(&player, "PlayPause")
}

pub fn mpris_next(player: String) -> Result<(), String> {
    call_player_method(&player, "Next")
}

pub fn mpris_previous(player: String) -> Result<(), String> {
    call_player_method(&player, "Previous")
}
