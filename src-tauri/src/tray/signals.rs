//! Las señales con las que los elementos de la bandeja avisan que algo cambió.
//!
//! Hasta acá el panel leía cada elemento una sola vez, al registrarse: un
//! Telegram con mensajes nuevos cambia el icono con `NewIcon`, y el panel seguía
//! mostrando el de antes hasta que se reiniciaba.
//!
//! Se escucha **una** regla por interfaz para todos los elementos, y no una por
//! elemento: las señales llegan con el nombre único del emisor y la ruta del
//! objeto, y con eso alcanza para saber de quién son (`TrayItem::unique_name` y
//! `object_path`). Así no hay tareas que cerrar cuando un elemento se va.
//!
//! - **StatusNotifierItem**: `NewIcon`, `NewOverlayIcon`, `NewAttentionIcon`,
//!   `NewToolTip`, `NewTitle` y `NewStatus` (ver [`Refresh`]).
//! - **dbusmenu**: `ItemsPropertiesUpdated` se aplica sobre el menú abierto
//!   (`SystrayPopupState`) sin volver a pedirlo; `LayoutUpdated` lo vuelve a
//!   pedir entero. Los dos avisan a la vista con `tray-popup-update`.

use crate::commands::load_dbus_menu_level;
use crate::logger::{log_debug, log_warning};
use crate::structs::{SystrayPopupState, TrayManager, TrayStatus};
use crate::tray::emit_tray_update;
use crate::tray::item_props::{refresh_item, Refresh, SNI_INTERFACES};
use crate::tray::menu_props::apply_items_properties_updated;
use crate::tray::sni_watcher::sni_proxy;
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, Manager};
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MatchRule, MessageStream, MessageType};

const DBUSMENU_INTERFACE: &str = "com.canonical.dbusmenu";

/// Cuánto se espera a que un elemento conteste lo que pide releer una señal.
const REFRESH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// Las claves de los elementos a los que les toca una señal.
pub fn items_for_signal(
    items: &HashMap<String, crate::structs::TrayItem>,
    sender: &str,
    path: &str,
) -> Vec<String> {
    items
        .iter()
        .filter(|(_, item)| item.unique_name.as_deref() == Some(sender) && item.object_path == path)
        .map(|(key, _)| key.clone())
        .collect()
}

async fn signal_stream(connection: &Connection, interface: &str) -> zbus::Result<MessageStream> {
    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface(interface)?
        .build();
    MessageStream::for_match_rule(rule, connection, None).await
}

/// Aplica una señal de un elemento. Devuelve si le tocó a alguno.
pub async fn apply_item_signal(
    connection: &Connection,
    tray_manager: &TrayManager,
    message: &zbus::Message,
) -> bool {
    let header = message.header();
    let (Some(sender), Some(path), Some(member)) =
        (header.sender(), header.path(), header.member())
    else {
        return false;
    };
    let Some(refresh) = Refresh::for_signal(member.as_str()) else {
        return false;
    };
    let (sender, path) = (sender.to_string(), path.to_string());

    let keys = {
        let items = tray_manager.read().await;
        items_for_signal(&items, &sender, &path)
    };
    if keys.is_empty() {
        return false;
    }

    // `NewStatus` trae el estado: no hace falta preguntarlo.
    let status = if refresh == Refresh::Status {
        message
            .body()
            .deserialize::<(String,)>()
            .ok()
            .map(|(s,)| TrayStatus::parse(&s))
    } else {
        None
    };

    for key in keys {
        let Some(mut item) = tray_manager.read().await.get(&key).cloned() else {
            continue;
        };
        match status.clone() {
            Some(status) => item.status = status,
            None => match sni_proxy(connection, &sender, &path).await {
                // Con tope: este bucle atiende a todos los elementos, y uno
                // colgado que no contesta no puede dejar quietos a los demás.
                Ok(proxy) => {
                    if tokio::time::timeout(
                        REFRESH_TIMEOUT,
                        refresh_item(&proxy, &mut item, refresh),
                    )
                    .await
                    .is_err()
                    {
                        log_debug(&format!("[SNI] {key} no contestó a tiempo"));
                        continue;
                    }
                }
                Err(e) => {
                    log_debug(&format!("[SNI] No se pudo releer {key}: {e}"));
                    continue;
                }
            },
        }
        // Se escribe sólo si el elemento sigue: pudo irse mientras se leía.
        let mut items = tray_manager.write().await;
        if let Some(slot) = items.get_mut(&key) {
            item.launcher = slot.launcher.clone();
            *slot = item;
        }
    }
    true
}

/// Si la señal de menú es del menú que está abierto, la aplica. Devuelve si
/// cambió lo que hay que dibujar.
pub async fn apply_menu_signal(
    connection: &Connection,
    tray_manager: &TrayManager,
    popup: &SystrayPopupState,
    message: &zbus::Message,
) -> bool {
    let header = message.header();
    let (Some(sender), Some(path), Some(member)) =
        (header.sender(), header.path(), header.member())
    else {
        return false;
    };
    let service_name = {
        let state = popup
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match state.as_ref() {
            Some(payload) => payload.service_name.clone(),
            None => return false,
        }
    };
    let (bus_name, menu_path) = {
        let items = tray_manager.read().await;
        let Some(item) = items
            .get(&service_name)
            .or_else(|| items.values().find(|i| i.service_name == service_name))
        else {
            return false;
        };
        if item.unique_name.as_deref() != Some(sender.as_str())
            || item.menu_path.as_deref() != Some(path.as_str())
        {
            return false;
        }
        (
            item.bus_name.clone().unwrap_or_else(|| sender.to_string()),
            path.to_string(),
        )
    };

    let changed = match member.as_str() {
        "ItemsPropertiesUpdated" => {
            type Updated = Vec<(i32, HashMap<String, OwnedValue>)>;
            type Removed = Vec<(i32, Vec<String>)>;
            let Ok((updated, removed)) = message.body().deserialize::<(Updated, Removed)>() else {
                log_debug("[dbusmenu] ItemsPropertiesUpdated con una firma inesperada");
                return false;
            };
            let mut state = popup
                .0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match state.as_mut() {
                Some(payload) if payload.service_name == service_name => {
                    apply_items_properties_updated(&mut payload.items, &updated, &removed)
                }
                _ => false,
            }
        }
        "LayoutUpdated" => match load_dbus_menu_level(connection, &bus_name, &menu_path, 0).await {
            Ok(items) => {
                let mut state = popup
                    .0
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                match state.as_mut() {
                    Some(payload) if payload.service_name == service_name => {
                        payload.items = items;
                        true
                    }
                    _ => false,
                }
            }
            Err(e) => {
                log_debug(&format!("[dbusmenu] No se pudo releer el menú: {e}"));
                false
            }
        },
        _ => false,
    };

    changed
}

/// Escucha las señales de todos los elementos y de sus menús.
pub async fn start(
    connection: Connection,
    tray_manager: TrayManager,
    app_handle: AppHandle,
) -> zbus::Result<()> {
    for interface in SNI_INTERFACES {
        let mut stream = signal_stream(&connection, interface).await?;
        let (connection, tray_manager, app_handle) =
            (connection.clone(), tray_manager.clone(), app_handle.clone());
        tokio::spawn(async move {
            while let Some(message) = stream.next().await {
                if let Ok(message) = message {
                    if apply_item_signal(&connection, &tray_manager, &message).await {
                        emit_tray_update(&app_handle).await;
                    }
                }
            }
        });
    }

    let mut menus = signal_stream(&connection, DBUSMENU_INTERFACE).await?;
    tokio::spawn(async move {
        while let Some(message) = menus.next().await {
            let Ok(message) = message else { continue };
            let Some(popup) = app_handle.try_state::<SystrayPopupState>() else {
                continue;
            };
            if apply_menu_signal(&connection, &tray_manager, &popup, &message).await {
                if let Err(e) = app_handle.emit("tray-popup-update", ()) {
                    log_warning(&format!("[dbusmenu] No se pudo avisar a la vista: {e}"));
                }
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structs::{TrayCategory, TrayItem};

    fn item(unique: &str, path: &str) -> TrayItem {
        TrayItem {
            id: "x".into(),
            service_name: "x".into(),
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
            object_path: path.into(),
            unique_name: Some(unique.into()),
            pid: None,
        }
    }

    #[test]
    fn la_senal_le_toca_al_elemento_de_ese_emisor_y_esa_ruta() {
        let mut items = HashMap::new();
        // Chrome publica varios elementos desde la misma conexión.
        items.insert("a".to_string(), item(":1.7", "/StatusNotifierItem/1"));
        items.insert("b".to_string(), item(":1.7", "/StatusNotifierItem/2"));
        items.insert("c".to_string(), item(":1.8", "/StatusNotifierItem"));
        assert_eq!(
            items_for_signal(&items, ":1.7", "/StatusNotifierItem/2"),
            vec!["b".to_string()]
        );
        assert!(items_for_signal(&items, ":1.9", "/StatusNotifierItem").is_empty());
    }
}
