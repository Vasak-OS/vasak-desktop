use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::thread;
use tauri::{AppHandle, Emitter};
use tokio::runtime::Builder as TokioBuilder;
use zbus::blocking::fdo::DBusProxy;
use zbus::blocking::{Connection, Proxy};
use zbus::fdo::DBusProxy as AsyncDBusProxy;
use zbus::zvariant::Value;
use zbus::MessageStream;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album_art_url: Option<String>,
    pub player: Option<String>,
    pub status: Option<String>,
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

// Helper blocking: consulta MPRIS para un servicio concreto y devuelve JSON con los campos que encuentre.
fn fetch_now_playing() -> Result<serde_json::Value, String> {
    eprintln!("[music] fetch_now_playing: start");
    let conn = Connection::session().map_err(|e| {
        eprintln!(
            "[music] fetch_now_playing: Connection::session error: {}",
            e
        );
        format!("zbus connect error: {}", e)
    })?;
    let dbus = DBusProxy::new(&conn).map_err(|e| format!("DBus proxy creation failed: {}", e))?;
    let owned_names = dbus.list_names().map_err(|e| {
        eprintln!("[music] fetch_now_playing: ListNames error: {}", e);
        format!("ListNames failed: {}", e)
    })?;
    let names: Vec<String> = owned_names.into_iter().map(|n| n.to_string()).collect();
    eprintln!("[music] fetch_now_playing: found {} bus names", names.len());

    // Buscar reproductor que esté reproduciendo; si no, tomar el primero con metadata.
    let mut fallback: Option<(String, Option<String>)> = None; // (service, status)
    for name in names
        .into_iter()
        .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
    {
        eprintln!("[music] fetch_now_playing: inspecting service {}", name);
        let proxy = Proxy::new(
            &conn,
            name.as_str(),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        );
        if let Ok(p) = proxy {
            // PlaybackStatus
            let status: Option<String> = p.get_property("PlaybackStatus").ok();
            eprintln!(
                "[music] fetch_now_playing: service {} PlaybackStatus={:?}",
                name, status
            );
            // Preferir Playing
            if status.as_deref() == Some("Playing") {
                eprintln!(
                    "[music] fetch_now_playing: service {} is Playing, extracting metadata",
                    name
                );
                // intentar metadata
                let title = p.get_property::<Value>("Metadata").ok().and_then(|v| {
                    serde_json::to_value(&v).ok().and_then(|j| {
                        j.get("xesam:title").and_then(|t| {
                            if t.is_array() {
                                t.get(0).and_then(|s| s.as_str()).map(|s| s.to_string())
                            } else {
                                t.as_str().map(|s| s.to_string())
                            }
                        })
                    })
                });
                let artist = p.get_property::<Value>("Metadata").ok().and_then(|v| {
                    serde_json::to_value(&v).ok().and_then(|j| {
                        j.get("xesam:artist").and_then(|a| {
                            if a.is_array() {
                                let arr: Vec<String> = a
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .filter_map(|it| it.as_str().map(|s| s.to_string()))
                                    .collect();
                                if !arr.is_empty() {
                                    Some(arr.join("; "))
                                } else {
                                    None
                                }
                            } else {
                                a.as_str().map(|s| s.to_string())
                            }
                        })
                    })
                });
                let art_url = p.get_property::<Value>("Metadata").ok().and_then(|v| {
                    serde_json::to_value(&v).ok().and_then(|j| {
                        j.get("mpris:artUrl")
                            .and_then(|u| u.as_str().map(|s| s.to_string()))
                    })
                });
                return Ok(json!({
                    "title": title.unwrap_or_else(|| "Unknown".to_string()),
                    "artist": artist.unwrap_or_else(|| "Unknown".to_string()),
                    "artUrl": art_url.unwrap_or_else(|| "".to_string()),
                    "player": name,
                    "status": status,
                }));
            } else if fallback.is_none() {
                eprintln!(
                    "[music] fetch_now_playing: service {} not Playing, store fallback",
                    name
                );
                fallback = Some((name.clone(), status));
            }
        }
    }

    // Si no hay ninguno Playing, intentar fallback
    if let Some((name, status)) = fallback {
        eprintln!("[music] fetch_now_playing: using fallback service {}", name);
        // reconectar proxy y tratar de extraer metadata (si falla, devolver solo player/status)
        let proxy = Proxy::new(
            &conn,
            name.as_str(),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .map_err(|e| format!("Proxy for fallback failed: {}", e))?;
        let title = proxy.get_property::<Value>("Metadata").ok().and_then(|v| {
            serde_json::to_value(&v).ok().and_then(|j| {
                j.get("xesam:title").and_then(|t| {
                    if t.is_array() {
                        t.get(0).and_then(|s| s.as_str()).map(|s| s.to_string())
                    } else {
                        t.as_str().map(|s| s.to_string())
                    }
                })
            })
        });
        let artist = proxy.get_property::<Value>("Metadata").ok().and_then(|v| {
            serde_json::to_value(&v).ok().and_then(|j| {
                j.get("xesam:artist").and_then(|a| {
                    if a.is_array() {
                        let arr: Vec<String> = a
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter_map(|it| it.as_str().map(|s| s.to_string()))
                            .collect();
                        if !arr.is_empty() {
                            Some(arr.join("; "))
                        } else {
                            None
                        }
                    } else {
                        a.as_str().map(|s| s.to_string())
                    }
                })
            })
        });
        let art_url = proxy.get_property::<Value>("Metadata").ok().and_then(|v| {
            serde_json::to_value(&v).ok().and_then(|j| {
                j.get("mpris:artUrl")
                    .and_then(|u| u.as_str().map(|s| s.to_string()))
            })
        });
        return Ok(json!({
            "title": title.unwrap_or_else(|| "Unknown".to_string()),
            "artist": artist.unwrap_or_else(|| "Unknown".to_string()),
            "artUrl": art_url.unwrap_or_else(|| "".to_string()),
            "player": name,
            "status": status,
        }));
    }

    // Ningún reproductor MPRIS encontrado
    eprintln!("[music] fetch_now_playing: no players found, returning Nothing playing");
    Ok(json!({ "title": "Nothing playing" }))
}

/// Arranca un monitor que escucha señales D-Bus (PropertiesChanged) y emite eventos Tauri.
/// Usa un runtime tokio en un hilo separado y la API async de zbus para recibir mensajes.
pub fn start_mpris_monitor(app: AppHandle) {
    // Arranca un hilo con un runtime tokio que escucha señales PropertiesChanged y emite eventos Tauri.
    let app_for_thread = app.clone();
    thread::spawn(move || {
        eprintln!("[music] start_mpris_monitor: spawning tokio runtime thread");
        // Crear runtime single-threaded para uso local
        let rt = TokioBuilder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let _ = rt.block_on(async move {
            if let Err(e) = monitor_signals_async(app_for_thread.clone()).await {
                eprintln!(
                    "[music] start_mpris_monitor: monitor_signals_async error: {:?}",
                    e
                );
                let _ = app_for_thread.emit("mpris-error", format!("monitor error: {}", e));
            }
        });
    });
}

// Función async que se suscribe a PropertiesChanged y emite actualizaciones al frontend
async fn monitor_signals_async(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let conn = zbus::Connection::session().await?;
    eprintln!("[music] monitor_signals_async: connected to session bus");

    // Registrar match rule vía org.freedesktop.DBus.AddMatch para asegurarnos de recibir las señales.
    let match_rule =
        "type='signal',interface='org.freedesktop.DBus.Properties',member='PropertiesChanged'";
    eprintln!(
        "[music] monitor_signals_async: adding match rule: {}",
        match_rule
    );
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

    // Crear proxy async para org.freedesktop.DBus (usado para resolver owners)
    let dbus_async = AsyncDBusProxy::new(&conn).await?;
    eprintln!("[music] monitor_signals_async: created AsyncDBusProxy for owner resolution");

    // Stream de mensajes del bus (usar una clon de la conexión para el stream)
    let stream_conn = conn.clone();
    let mut stream = MessageStream::from(stream_conn);
    while let Some(msg_res) = stream.next().await {
        if let Ok(m) = msg_res {
            // obtener el header (no es Option)
            let header = m.header();
            // Filtrar por interfaz + member y loggear el mensaje recibido
            let iface = header.interface().map(|i| i.as_str().to_string());
            let member = header.member().map(|me| me.as_str().to_string());
            let sender_opt = header.sender().map(|s| s.to_string());
            eprintln!(
                "[music] monitor_signals_async: got message iface={:?} member={:?} sender={:?}",
                iface, member, sender_opt
            );
            if iface.as_deref() == Some("org.freedesktop.DBus.Properties")
                && member.as_deref() == Some("PropertiesChanged")
            {
                if let Some(sender_str) = sender_opt {
                    eprintln!(
                        "[music] monitor_signals_async: PropertiesChanged from {}",
                        sender_str
                    );
                    // Si sender ya es well-known MPRIS, proceder.
                    let mut should_requery = sender_str.starts_with("org.mpris.MediaPlayer2.");
                    // Si es un unique name (:1.xxx), intentar resolver a un well-known org.mpris.*
                    if !should_requery && sender_str.starts_with(':') {
                        eprintln!(
                            "[music] monitor_signals_async: resolving unique name {} to well-known",
                            sender_str
                        );
                        // listar nombres (devuelve OwnedBusName) y comprobar owner para cada org.mpris.*
                        match dbus_async.list_names().await {
                            Ok(names) => {
                                for owned in names
                                    .into_iter()
                                    .filter(|n| n.as_str().starts_with("org.mpris.MediaPlayer2."))
                                {
                                    // owned es OwnedBusName; convertir a BusName aceptado por get_name_owner
                                    let bus_name = owned.clone().into();
                                    eprintln!(
                                        "[music] monitor_signals_async: checking owned name {}",
                                        owned.as_str()
                                    );
                                    match dbus_async.get_name_owner(bus_name).await {
                                        Ok(owner) => {
                                            eprintln!(
                                                "[music] monitor_signals_async: name {} owner {}",
                                                owned.as_str(),
                                                owner.to_string()
                                            );
                                            if owner.to_string() == sender_str {
                                                eprintln!("[music] monitor_signals_async: resolved {} -> {}", sender_str, owned.as_str());
                                                should_requery = true;
                                                break;
                                            }
                                        }
                                        Err(err) => {
                                            eprintln!("[music] monitor_signals_async: GetNameOwner failed for {}: {:?}", owned.as_str(), err);
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                eprintln!("[music] monitor_signals_async: list_names failed during resolve: {:?}", err);
                            }
                        }
                    }

                    if should_requery {
                        eprintln!("[music] monitor_signals_async: will re-query state due to signal from {}", sender_str);
                        let app_clone = app.clone();
                        let info_res = tokio::task::spawn_blocking(|| fetch_now_playing()).await;
                        eprintln!(
                            "[music] monitor_signals_async: re-query finished: {:?}",
                            info_res
                        );
                        if let Ok(Ok(info)) = info_res {
                            eprintln!(
                                "[music] monitor_signals_async: emitting music-playing-update"
                            );
                            let _ = app_clone.emit("music-playing-update", info);
                        } else if let Ok(Err(e)) = info_res {
                            eprintln!(
                                "[music] monitor_signals_async: fetch_now_playing error: {}",
                                e
                            );
                            let _ = app_clone.emit("mpris-error", e);
                        } else if let Err(join_err) = info_res {
                            eprintln!(
                                "[music] monitor_signals_async: spawn_blocking join error: {:?}",
                                join_err
                            );
                            let _ = app_clone
                                .emit("mpris-error", format!("join error: {:?}", join_err));
                        }
                    } else {
                        eprintln!(
                            "[music] monitor_signals_async: signal from {} is not MPRIS (ignored)",
                            sender_str
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
