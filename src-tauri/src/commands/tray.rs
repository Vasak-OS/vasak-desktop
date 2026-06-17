use crate::structs::{SystrayPopupPayload, SystrayPopupState, TrayItem, TrayManager, TrayMenu};
use crate::tray::sni_watcher::SniWatcher;
use crate::tray::sni_item::SniItemProxy;
use crate::tray::dbus_menu::{DbusMenuProxy, DbusMenuLayout, call_get_layout, call_about_to_show};
use crate::app_url::get_app_url;
use crate::logger::{log_info, log_error, log_debug};
use crate::monitor_manager::get_primary_monitor;
use crate::windows_apps::create_systray_popup_window;
use futures_util::future::BoxFuture;
use tauri::Manager;
use tauri::{PhysicalPosition, Position, Size, Url};
use zbus::Connection;
use zbus::zvariant::Value;

async fn resolve_tray_item(
    tray_manager: &TrayManager,
    service_name: &str,
) -> Option<TrayItem> {
    let manager = tray_manager.read().await;

    manager
        .get(service_name)
        .cloned()
        .or_else(|| {
            manager
                .values()
                .find(|item| {
                    item.service_name == service_name
                        || item.bus_name.as_deref() == Some(service_name)
                })
                .cloned()
        })
}

    fn resolve_tray_bus_name<'a>(tray_item: &'a TrayItem, service_name: &'a str) -> Result<&'a str, String> {
        tray_item
        .bus_name
        .as_deref()
        .filter(|name| !name.is_empty())
        .or_else(|| if !service_name.starts_with('/') { Some(service_name) } else { None })
        .ok_or_else(|| format!("No bus name available for tray item {}", service_name))
    }

#[tauri::command]
pub async fn init_sni_watcher(
    app_handle: tauri::AppHandle,
    tray_manager: tauri::State<'_, TrayManager>,
) -> Result<(), String> {
    log_info("Inicializando SNI watcher para sistema de bandeja");
    let manager = tray_manager.inner().clone();
    let watcher = SniWatcher::new(manager, app_handle)
        .await
        .map_err(|e| {
            log_error(&format!("Error inicializando SNI watcher: {}", e));
            format!("Error inicializando SNI watcher: {}", e)
        })?;

    watcher
        .start_watching()
        .await
        .map_err(|e| {
            log_error(&format!("Error iniciando watcher: {}", e));
            format!("Error iniciando watcher: {}", e)
        })?;

    log_info("SNI watcher iniciado correctamente");
    Ok(())
}

#[tauri::command]
pub async fn get_tray_items(
    tray_manager: tauri::State<'_, TrayManager>,
) -> Result<Vec<TrayItem>, String> {
    log_debug("Obteniendo items de la bandeja del sistema");
    let manager = tray_manager.read().await;
    let items: Vec<TrayItem> = manager.values().cloned().collect();
    log_debug(&format!("Obtenidos {} items de bandeja", items.len()));
    Ok(items)
}

async fn get_sni_proxy<'a>(conn: &'a Connection, service_name: &'a str) -> Result<SniItemProxy<'a>, String> {
    let (bus_name, object_path) = if service_name.contains('/') {
        let parts: Vec<&str> = service_name.splitn(2, '/').collect();
        (parts[0], format!("/{}", parts[1]))
    } else {
        (service_name, "/StatusNotifierItem".to_string())
    };

    SniItemProxy::builder(conn)
        .destination(bus_name).map_err(|e| e.to_string())?
        .path(object_path).map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tray_item_activate(service_name: String, x: i32, y: i32) -> Result<(), String> {
    log_info(&format!("Activando item de bandeja: {} en ({}, {})", service_name, x, y));
    let conn = Connection::session().await.map_err(|e| {
        log_error(&format!("Error conectando a D-Bus session: {}", e));
        e.to_string()
    })?;
    let proxy = get_sni_proxy(&conn, &service_name).await?;
    
    proxy.activate(x, y).await.map_err(|e| {
        log_error(&format!("Error activando item '{}': {}", service_name, e));
        e.to_string()
    })?;
    log_debug(&format!("Item '{}' activado correctamente", service_name));
    Ok(())
}

#[tauri::command]
pub async fn tray_item_secondary_activate(
    service_name: String,
    x: i32,
    y: i32,
) -> Result<(), String> {
    log_info(&format!("Activación secundaria de item de bandeja: {} en ({}, {})", service_name, x, y));
    let conn = Connection::session().await.map_err(|e| e.to_string())?;
    let proxy = get_sni_proxy(&conn, &service_name).await?;
    
    proxy.secondary_activate(x, y).await.map_err(|e| {
        log_error(&format!("Error en activación secundaria de '{}': {}", service_name, e));
        e.to_string()
    })?;
    log_debug(&format!("Activación secundaria de '{}' correcta", service_name));
    Ok(())
}

fn get_string(v: &Value) -> String {
    match v {
        Value::Str(s) => s.as_str().to_string(),
        Value::Value(inner) => get_string(inner.as_ref()),
        _ => String::new(),
    }
}

fn get_bool(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Value(inner) => get_bool(inner.as_ref()),
        _ => true, // default
    }
}

fn get_i32(v: &Value) -> Option<i32> {
    match v {
        Value::I32(i) => Some(*i),
        Value::Value(inner) => get_i32(inner.as_ref()),
        _ => None,
    }
}

// Helper to extract properties from zvariant::Dict
fn extract_props_from_dict(dict: &zbus::zvariant::Dict) -> (String, bool, bool, String, Option<String>, Option<bool>) {
    let mut label = String::new();
    let mut enabled = true;
    let mut visible = true;
    let mut menu_type = "standard".to_string();
    let mut icon_name = None;
    let mut checked = None;

    for (k, v) in dict.iter() {
             // k and v are &Value
             let key_str = match k {
                 Value::Str(s) => s.as_str(),
                 _ => continue,
             };

             match key_str {
                 "label" => label = get_string(v),
                 "enabled" => enabled = get_bool(v),
                 "visible" => visible = get_bool(v),
                 "type" => menu_type = get_string(v),
                 "children-display" => {
                     let child_display = get_string(v);
                     if child_display == "submenu" {
                         menu_type = "submenu".to_string();
                     }
                 },
                 "icon-name" => icon_name = Some(get_string(v)),
                 "toggle-state" => {
                     if let Some(i) = get_i32(v) {
                         checked = Some(i == 1);
                     }
                 },
                 _ => {}
             }
        }
    
    (label, enabled, visible, menu_type, icon_name, checked)
}

fn parse_dbus_menu_value(v: &Value) -> Option<TrayMenu> {
    let s = match v {
        Value::Structure(s) => s,
        _ => return None,
    };
    
    let fields = s.fields();
    if fields.len() < 3 { return None; }
    
    let id = get_i32(&fields[0]).unwrap_or(0);
    
    let (label, enabled, visible, menu_type, icon_name, checked) = match &fields[1] {
        Value::Dict(d) => extract_props_from_dict(d),
        _ => (String::new(), true, true, "standard".to_string(), None, None),
    };

    let mut children = Vec::new();
    if let Value::Array(a) = &fields[2] {
        for child in a.iter() {
            if let Some(menu) = parse_dbus_menu_value(child) {
                children.push(menu);
            }
        }
    }

    Some(TrayMenu {
        id,
        label: label.replace("_", ""), 
        enabled,
        visible,
        menu_type,
        checked,
        icon: icon_name,
        children: if children.is_empty() { None } else { Some(children) },
    })
}

fn load_dbus_menu_level<'a>(
    conn: &'a Connection,
    bus_name: &'a str,
    menu_path: &'a str,
    parent_id: i32,
) -> BoxFuture<'a, Result<Vec<TrayMenu>, String>> {
    Box::pin(async move {
        let _ = call_about_to_show(conn, bus_name, menu_path, parent_id).await;

        let (_revision, layout) = call_get_layout(conn, bus_name, menu_path).await.map_err(|e| e.to_string())?;
        let root_menu = parse_dbus_menu_layout(layout);
        let mut items = root_menu.children.unwrap_or_default();

        for item in items.iter_mut() {
            if item.menu_type == "submenu" {
                match load_dbus_menu_level(conn, bus_name, menu_path, item.id).await {
                    Ok(children) if !children.is_empty() => {
                        item.children = Some(children);
                    }
                    Ok(_) => {}
                    Err(error) => {
                        log_error(&format!("[fetch_dbus_menu] Failed to load submenu {}: {}", item.id, error));
                    }
                }
            }
        }

        Ok(items)
    })
}

fn parse_dbus_menu_layout(layout: DbusMenuLayout) -> TrayMenu {
    let DbusMenuLayout(id, props, children_variants) = layout;

    let label = props.get("label").map(|v| get_string(v)).unwrap_or_default();
    let enabled = props.get("enabled").map(|v| get_bool(v)).unwrap_or(true);
    let visible = props.get("visible").map(|v| get_bool(v)).unwrap_or(true);
    let menu_type = props.get("type").map(|v| get_string(v)).unwrap_or_else(|| "standard".to_string());
    let icon_name = props.get("icon-name").map(|v| get_string(v));
    
    let checked = props.get("toggle-state").and_then(|v| get_i32(v)).map(|toggle_state| toggle_state == 1);

    let mut children: Vec<TrayMenu> = Vec::new();

    for child_variant in children_variants {
        // child_variant is OwnedValue -> &Value
        if let Some(child_menu) = parse_dbus_menu_value(&child_variant) {
             children.push(child_menu);
        }
    }

    TrayMenu {
        id,
        label: label.replace("_", ""), 
        enabled,
        visible,
        menu_type,
        checked,
        icon: icon_name,
        children: if children.is_empty() { None } else { Some(children) },
    }
}

#[tauri::command]
pub async fn get_tray_menu(
    service_name: String,
    tray_manager: tauri::State<'_, TrayManager>,
) -> Result<Vec<TrayMenu>, String> {
    let tray_item = resolve_tray_item(&tray_manager, &service_name)
        .await
        .ok_or("No tray item found")?;

    let menu_path = tray_item
        .menu_path
        .clone()
        .ok_or("No menu path available for this item")?;
    let bus_name = resolve_tray_bus_name(&tray_item, &service_name)?;

    let conn = Connection::session().await.map_err(|e| e.to_string())?;

    load_dbus_menu_level(&conn, &bus_name, &menu_path, 0).await
}


#[tauri::command]
pub async fn tray_menu_item_click(
    service_name: String, 
    menu_id: i32,
    tray_manager: tauri::State<'_, TrayManager>,
) -> Result<(), String> {
    let tray_item = resolve_tray_item(&tray_manager, &service_name)
        .await
        .ok_or("No tray item found")?;

    let menu_path = tray_item
        .menu_path
        .clone()
        .ok_or("No menu path available for this item")?;
    let bus_name = resolve_tray_bus_name(&tray_item, &service_name)?;
    let conn = Connection::session().await.map_err(|e| e.to_string())?;

    let proxy = DbusMenuProxy::builder(&conn)
        .destination(bus_name).map_err(|e| e.to_string())?
        .path(menu_path).map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;

    // timestamp 0, event "clicked"
    proxy.event(menu_id, "clicked", &Value::from(""), 0).await.map_err(|e| e.to_string())?;

    Ok(())
}

async fn fetch_dbus_menu(bus_name: &str, service_name: &str, menu_path: &str) -> Result<Vec<TrayMenu>, String> {
    log_info(&format!("[fetch_dbus_menu] Connecting to session bus for {}", service_name));
    let conn = Connection::session().await.map_err(|e| {
        log_error(&format!("[fetch_dbus_menu] DBus connection error: {}", e));
        e.to_string()
    })?;

    log_info(&format!("[fetch_dbus_menu] Building proxy for {} path {} menu_path {}", service_name, bus_name, menu_path));
    let items = load_dbus_menu_level(&conn, bus_name, menu_path, 0).await.map_err(|e| {
        log_error(&format!("[fetch_dbus_menu] Failed to load menu tree: {}", e));
        e
    })?;

    let item_count = items.len();
    log_info(&format!("[fetch_dbus_menu] Parsed {} menu items for {}", item_count, service_name));
    Ok(items)
}

#[tauri::command]
pub async fn open_tray_popup(
    service_name: String,
    app: tauri::AppHandle,
    tray_manager: tauri::State<'_, TrayManager>,
    popup_state: tauri::State<'_, SystrayPopupState>,
) -> Result<(), String> {
    log_info(&format!("Opening tray popup for: {}", service_name));

    let tray_item = resolve_tray_item(&tray_manager, &service_name)
        .await
        .ok_or("No tray item found")?;

    let menu_path = tray_item.menu_path.clone();
    let bus_name = resolve_tray_bus_name(&tray_item, &service_name)?;
    let title = tray_item.title.clone().unwrap_or_default();
    let icon_id = tray_item.icon_name.clone().unwrap_or_default();
    let icon_data = tray_item.icon_data.clone();
    let tooltip = tray_item.tooltip.clone();
    let status = Some(tray_item.status.clone());

    let items = if let Some(ref path) = menu_path {
        log_info(&format!("[open_tray_popup] Fetching DBus menu for {} at {}", service_name, path));
        match fetch_dbus_menu(bus_name, &service_name, path).await {
            Ok(items) => {
                log_info(&format!("[open_tray_popup] Fetched {} menu items", items.len()));
                items
            },
            Err(e) => {
                log_error(&format!("[open_tray_popup] Failed to fetch tray menu: {}", e));
                Vec::new()
            }
        }
    } else {
        log_info(&format!("[open_tray_popup] No menu_path for {}, showing empty popup", service_name));
        Vec::new()
    };

    let payload = SystrayPopupPayload {
        icon_id,
        icon_data,
        tooltip,
        status,
        title,
        service_name: service_name.clone(),
        items: items.clone(),
    };

    *popup_state.0.lock().unwrap() = Some(payload);

    let popup_width = 700.0;
    let popup_height = 620.0;

    if let Some(window) = app.get_webview_window("systray_popup") {
        let complete_url = format!("{}/index.html#/applets/tray-popup", get_app_url());
        if let Ok(url) = Url::parse(&complete_url) {
            let _ = window.navigate(url);
        }
        let _ = window.set_size(Size::Physical(tauri::PhysicalSize { width: popup_width as u32, height: popup_height as u32 }));
        if let Some(monitor) = get_primary_monitor(&app) {
            let pos = monitor.position();
            let size = monitor.size();
            let cx = pos.x + (size.width as i32 / 2) - (popup_width as i32 / 2);
            let cy = pos.y + (size.height as i32 / 2) - (popup_height as i32 / 2);
            let _ = window.set_position(Position::Physical(PhysicalPosition { x: cx, y: cy }));
        }
        let _ = window.set_focus();
        let _ = window.show();
        log_info("Reused existing systray popup window");
        return Ok(());
    }

    create_systray_popup_window(app, popup_width, popup_height)
        .await
        .map_err(|e| format!("Failed to create systray popup: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_tray_popup_data(
    popup_state: tauri::State<'_, SystrayPopupState>,
) -> Result<Option<SystrayPopupPayload>, String> {
    let data = popup_state.0.lock().unwrap().clone();
    Ok(data)
}

#[tauri::command]
pub async fn tray_popup_click(
    menu_id: i32,
    tray_manager: tauri::State<'_, TrayManager>,
    popup_state: tauri::State<'_, SystrayPopupState>,
) -> Result<(), String> {
    let service_name = {
        let data = popup_state.0.lock().unwrap();
        data.as_ref().map(|d| d.service_name.clone())
    }.ok_or("No popup data available")?;

    let tray_item = resolve_tray_item(&tray_manager, &service_name)
        .await
        .ok_or("No tray item found")?;

    let menu_path = tray_item
        .menu_path
        .clone()
        .ok_or("No menu path available for this item")?;
    let bus_name = resolve_tray_bus_name(&tray_item, &service_name)?;
    let conn = Connection::session().await.map_err(|e| e.to_string())?;

    let proxy = DbusMenuProxy::builder(&conn)
        .destination(bus_name).map_err(|e| e.to_string())?
        .path(menu_path).map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;

    proxy.event(menu_id, "clicked", &Value::from(""), 0).await.map_err(|e| e.to_string())?;

    Ok(())
}
