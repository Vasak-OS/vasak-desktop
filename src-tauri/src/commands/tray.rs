use crate::structs::{TrayItem, TrayManager, TrayMenu};
use crate::tray::sni_watcher::SniWatcher;
use crate::tray::sni_item::SniItemProxy;
use crate::tray::dbus_menu::{DbusMenuProxy, DbusMenuLayout};
use crate::logger::{log_info, log_error, log_debug};
use zbus::Connection;
use zbus::zvariant::Value;

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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
fn get_string(v: &Value) -> String {
    match v {
        Value::Str(s) => s.as_str().to_string(),
        _ => String::new(),
    }
}

#[allow(dead_code)]
fn get_bool(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        _ => true, // default
    }
}

#[allow(dead_code)]
fn get_i32(v: &Value) -> Option<i32> {
    match v {
        Value::I32(i) => Some(*i),
        _ => None,
    }
}

// Helper to extract properties from zvariant::Dict
// Helper to extract properties from zvariant::Dict
#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
#[tauri::command]
pub async fn get_tray_menu(
    service_name: String,
    tray_manager: tauri::State<'_, TrayManager>,
) -> Result<Vec<TrayMenu>, String> {
    let menu_path_opt = {
        let manager = tray_manager.read().await;
        manager.get(&service_name).and_then(|item| item.menu_path.clone())
    };

    let menu_path = menu_path_opt.ok_or("No menu path available for this item")?;

    let conn = Connection::session().await.map_err(|e| e.to_string())?;
    
    // The menu service usually resides on the same bus name as the item
    // but the object path is specific (menu_path)
    let bus_name = if service_name.contains('/') {
        service_name.split('/').next().ok_or("Invalid service name format")?
    } else {
        &service_name
    };

    let proxy = DbusMenuProxy::builder(&conn)
        .destination(bus_name).map_err(|e| e.to_string())?
        .path(menu_path).map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;

    // propertyNames empty = get all
    let (_revision, layout) = proxy.get_layout(0, -1, &[]).await.map_err(|e| e.to_string())?;

    // The root layout usually isn't a visible item but contains the children
    // Parse the root and return its children
    let root_menu = parse_dbus_menu_layout(layout);
    
    Ok(root_menu.children.unwrap_or_default())
}


#[allow(dead_code)]
#[tauri::command]
pub async fn tray_menu_item_click(
    service_name: String, 
    menu_id: i32,
    tray_manager: tauri::State<'_, TrayManager>,
) -> Result<(), String> {
     let menu_path_opt = {
        let manager = tray_manager.read().await;
        manager.get(&service_name).and_then(|item| item.menu_path.clone())
    };

    let menu_path = menu_path_opt.ok_or("No menu path available for this item")?;
    let conn = Connection::session().await.map_err(|e| e.to_string())?;

    let bus_name = if service_name.contains('/') {
        service_name.split('/').next().ok_or("Invalid service name format")?
    } else {
        &service_name
    };

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
