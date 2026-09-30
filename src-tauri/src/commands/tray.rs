use crate::logger::{log_debug, log_error, log_info};
use crate::monitor_manager::get_primary_monitor;
use crate::panel_position;
use crate::structs::{SystrayPopupPayload, SystrayPopupState, TrayItem, TrayManager, TrayMenu};
use crate::tray::dbus_menu::{call_about_to_show, call_get_layout, DbusMenuLayout, DbusMenuProxy};
use crate::tray::sni_item::SniItemProxy;
use crate::tray::sni_watcher::SniWatcher;
use crate::windows_apps::anchored_applet::{open_anchored_applet, AnchorRect};
use futures_util::future::BoxFuture;
use zbus::zvariant::Value;
use zbus::Connection;

async fn resolve_tray_item(tray_manager: &TrayManager, service_name: &str) -> Option<TrayItem> {
    let manager = tray_manager.read().await;

    manager.get(service_name).cloned().or_else(|| {
        manager
            .values()
            .find(|item| {
                item.service_name == service_name || item.bus_name.as_deref() == Some(service_name)
            })
            .cloned()
    })
}

fn resolve_tray_bus_name<'a>(
    tray_item: &'a TrayItem,
    service_name: &'a str,
) -> Result<&'a str, String> {
    tray_item
        .bus_name
        .as_deref()
        .filter(|name| !name.is_empty())
        .or_else(|| {
            if !service_name.starts_with('/') {
                Some(service_name)
            } else {
                None
            }
        })
        .ok_or_else(|| format!("No bus name available for tray item {}", service_name))
}

#[tauri::command]
pub async fn init_sni_watcher(
    app_handle: tauri::AppHandle,
    tray_manager: tauri::State<'_, TrayManager>,
) -> Result<(), String> {
    log_info("Inicializando SNI watcher para sistema de bandeja");
    let manager = tray_manager.inner().clone();

    // No-op when the tray applet already started the watcher at boot; the panel
    // webview calls this again on every reload.
    SniWatcher::ensure_started(manager, app_handle)
        .await
        .map_err(|e| {
            log_error(&format!("Error inicializando SNI watcher: {}", e));
            format!("Error inicializando SNI watcher: {}", e)
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

async fn get_sni_proxy<'a>(
    conn: &'a Connection,
    service_name: &'a str,
) -> Result<SniItemProxy<'a>, String> {
    let (bus_name, object_path) = if service_name.contains('/') {
        let parts: Vec<&str> = service_name.splitn(2, '/').collect();
        (parts[0], format!("/{}", parts[1]))
    } else {
        (service_name, "/StatusNotifierItem".to_string())
    };

    SniItemProxy::builder(conn)
        .destination(bus_name)
        .map_err(|e| e.to_string())?
        .path(object_path)
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn tray_item_activate(service_name: String, x: i32, y: i32) -> Result<(), String> {
    log_info(&format!(
        "Activando item de bandeja: {} en ({}, {})",
        service_name, x, y
    ));
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
    log_info(&format!(
        "Activación secundaria de item de bandeja: {} en ({}, {})",
        service_name, x, y
    ));
    let conn = Connection::session().await.map_err(|e| e.to_string())?;
    let proxy = get_sni_proxy(&conn, &service_name).await?;

    proxy.secondary_activate(x, y).await.map_err(|e| {
        log_error(&format!(
            "Error en activación secundaria de '{}': {}",
            service_name, e
        ));
        e.to_string()
    })?;
    log_debug(&format!(
        "Activación secundaria de '{}' correcta",
        service_name
    ));
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
/// La etiqueta de una entrada de dbusmenu, sin la marca del atajo de teclado.
///
/// La especificación marca la letra del atajo con un guion bajo adelante
/// («_Salir») y escribe `__` para un guion bajo de verdad. El nivel superior del
/// menú los borraba todos —y con ellos el literal— y el de los hijos ninguno,
/// así que en los submenús se leía «_Salir».
fn menu_label(raw: &str) -> String {
    let mut label = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '_' {
            if chars.peek() == Some(&'_') {
                chars.next();
                label.push('_');
            }
        } else {
            label.push(c);
        }
    }
    label
}

/// Si una entrada está tildada, sólo si es tildable.
///
/// `toggle-state` no significa nada sin `toggle-type` («checkmark» o «radio»), y
/// hay programas que lo mandan en 0 o -1 en entradas comunes: tomarlo igual las
/// dibujaba como casillas destildadas.
fn menu_checked(toggle_type: Option<&str>, toggle_state: Option<i32>) -> Option<bool> {
    match toggle_type {
        Some("checkmark") | Some("radio") => Some(toggle_state == Some(1)),
        _ => None,
    }
}

fn extract_props_from_dict(
    dict: &zbus::zvariant::Dict,
) -> (String, bool, bool, String, Option<String>, Option<bool>) {
    let mut label = String::new();
    let mut enabled = true;
    let mut visible = true;
    let mut menu_type = "standard".to_string();
    let mut icon_name = None;
    let mut toggle_type = None;
    let mut toggle_state = None;

    for (k, v) in dict.iter() {
        // k and v are &Value
        let key_str = match k {
            Value::Str(s) => s.as_str(),
            _ => continue,
        };

        match key_str {
            "label" => label = menu_label(&get_string(v)),
            "enabled" => enabled = get_bool(v),
            "visible" => visible = get_bool(v),
            "type" => menu_type = get_string(v),
            "children-display" => {
                let child_display = get_string(v);
                if child_display == "submenu" {
                    menu_type = "submenu".to_string();
                }
            }
            "icon-name" => icon_name = Some(get_string(v)),
            "toggle-type" => toggle_type = Some(get_string(v)),
            "toggle-state" => toggle_state = get_i32(v),
            _ => {}
        }
    }

    let checked = menu_checked(toggle_type.as_deref(), toggle_state);
    (label, enabled, visible, menu_type, icon_name, checked)
}

fn parse_dbus_menu_value(v: &Value) -> Option<TrayMenu> {
    let s = match v {
        Value::Structure(s) => s,
        _ => return None,
    };

    let fields = s.fields();
    if fields.len() < 3 {
        return None;
    }

    let id = get_i32(&fields[0]).unwrap_or(0);

    let (label, enabled, visible, menu_type, icon_name, checked) = match &fields[1] {
        Value::Dict(d) => extract_props_from_dict(d),
        _ => (
            String::new(),
            true,
            true,
            "standard".to_string(),
            None,
            None,
        ),
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
        // Ya viene sin la marca del atajo: `extract_props_from_dict` la sacó.
        // Otra pasada convertía `mi__archivo` en `miarchivo`.
        label,
        enabled,
        visible,
        menu_type,
        checked,
        icon: icon_name,
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
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

        let (_revision, layout) = call_get_layout(conn, bus_name, menu_path, parent_id)
            .await
            .map_err(|e| e.to_string())?;
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
                        log_error(&format!(
                            "[fetch_dbus_menu] Failed to load submenu {}: {}",
                            item.id, error
                        ));
                    }
                }
            }
        }

        Ok(items)
    })
}

fn parse_dbus_menu_layout(layout: DbusMenuLayout) -> TrayMenu {
    let DbusMenuLayout(id, props, children_variants) = layout;

    let label = props
        .get("label")
        .map(|v| get_string(v))
        .unwrap_or_default();
    let enabled = props.get("enabled").map(|v| get_bool(v)).unwrap_or(true);
    let visible = props.get("visible").map(|v| get_bool(v)).unwrap_or(true);
    let menu_type = props
        .get("type")
        .map(|v| get_string(v))
        .unwrap_or_else(|| "standard".to_string());
    let icon_name = props.get("icon-name").map(|v| get_string(v));

    let toggle_type = props.get("toggle-type").map(|v| get_string(v));
    let checked = menu_checked(
        toggle_type.as_deref(),
        props.get("toggle-state").and_then(|v| get_i32(v)),
    );

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
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
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

    load_dbus_menu_level(&conn, bus_name, &menu_path, 0).await
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
        .destination(bus_name)
        .map_err(|e| e.to_string())?
        .path(menu_path)
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;

    // timestamp 0, event "clicked"
    proxy
        .event(menu_id, "clicked", &Value::from(""), 0)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

async fn fetch_dbus_menu(
    bus_name: &str,
    service_name: &str,
    menu_path: &str,
) -> Result<Vec<TrayMenu>, String> {
    log_info(&format!(
        "[fetch_dbus_menu] Connecting to session bus for {}",
        service_name
    ));
    let conn = Connection::session().await.map_err(|e| {
        log_error(&format!("[fetch_dbus_menu] DBus connection error: {}", e));
        e.to_string()
    })?;

    log_info(&format!(
        "[fetch_dbus_menu] Building proxy for {} path {} menu_path {}",
        service_name, bus_name, menu_path
    ));
    let items = load_dbus_menu_level(&conn, bus_name, menu_path, 0)
        .await
        .map_err(|e| {
            log_error(&format!(
                "[fetch_dbus_menu] Failed to load menu tree: {}",
                e
            ));
            e
        })?;

    let item_count = items.len();
    log_info(&format!(
        "[fetch_dbus_menu] Parsed {} menu items for {}",
        item_count, service_name
    ));
    Ok(items)
}

/// Las medidas del menú de la bandeja, en píxeles lógicos.
///
/// Tienen que coincidir con las clases de `TrayPopupView.vue`: una entrada es
/// `h-8`, un separador `my-1` más su línea, el título de un submenú `h-7`, y el
/// contenedor compacto de `AppletPopover` suma `p-1` y el borde de cada lado.
pub const TRAY_MENU_WIDTH: f64 = 280.0;
const TRAY_MENU_ITEM: f64 = 32.0;
const TRAY_MENU_SEPARATOR: f64 = 9.0;
const TRAY_MENU_CAPTION: f64 = 28.0;
const TRAY_MENU_CHROME: f64 = 10.0;
/// Hasta dónde crece antes de desplazarse. Un menú de treinta entradas no
/// puede tapar media pantalla para ser un menú.
const TRAY_MENU_MAX_HEIGHT: f64 = 560.0;

/// Un submenú se dibuja como título con sus entradas debajo, tenga o no hijos:
/// uno vacío no es una acción. Mismo criterio que `trayMenuRows`.
fn is_submenu(item: &TrayMenu) -> bool {
    item.menu_type == "submenu" || item.children.as_deref().is_some_and(|c| !c.is_empty())
}

fn tray_menu_rows(items: &[TrayMenu]) -> f64 {
    items
        .iter()
        .filter(|item| item.visible)
        .map(|item| {
            if item.menu_type == "separator" {
                TRAY_MENU_SEPARATOR
            } else if is_submenu(item) {
                TRAY_MENU_CAPTION + tray_menu_rows(item.children.as_deref().unwrap_or_default())
            } else {
                TRAY_MENU_ITEM
            }
        })
        .sum()
}

/// Cuánto mide el menú de un icono de la bandeja: lo que ocupan sus entradas
/// visibles, sin pasarse de [`TRAY_MENU_MAX_HEIGHT`]. Sin nada que tocar, el
/// menú es sólo el aviso de una línea, como lo dibuja la vista.
pub fn tray_menu_size(items: &[TrayMenu]) -> (f64, f64) {
    let rows = if has_visible_entries(items) {
        tray_menu_rows(items)
    } else {
        TRAY_MENU_ITEM
    };
    (
        TRAY_MENU_WIDTH,
        (TRAY_MENU_CHROME + rows).min(TRAY_MENU_MAX_HEIGHT),
    )
}

/// Si el menú tiene alguna entrada que se pueda tocar, a cualquier profundidad.
/// Los separadores y los títulos de submenú no cuentan. Mismo criterio que
/// `hasActions` en `tools/tray-menu.ts`.
fn has_visible_entries(items: &[TrayMenu]) -> bool {
    items.iter().filter(|item| item.visible).any(|item| {
        if item.menu_type == "separator" {
            false
        } else if is_submenu(item) {
            has_visible_entries(item.children.as_deref().unwrap_or_default())
        } else {
            true
        }
    })
}

/// El centro del botón en coordenadas de pantalla.
///
/// `AnchorRect` se mide dentro del panel; `ContextMenu` pide la posición en la
/// pantalla, así que se suma dónde empieza el panel: con el panel abajo o a la
/// derecha, el centro local caía en la otra punta.
fn anchor_screen_center(rect: &AnchorRect, panel_origin: (f64, f64)) -> (i32, i32) {
    (
        (panel_origin.0 + rect.x + rect.width / 2.0).round() as i32,
        (panel_origin.1 + rect.y + rect.height / 2.0).round() as i32,
    )
}

#[tauri::command]
pub async fn open_tray_popup(
    service_name: String,
    anchor: Option<AnchorRect>,
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
        log_info(&format!(
            "[open_tray_popup] Fetching DBus menu for {} at {}",
            service_name, path
        ));
        match fetch_dbus_menu(bus_name, &service_name, path).await {
            Ok(items) => {
                log_info(&format!(
                    "[open_tray_popup] Fetched {} menu items",
                    items.len()
                ));
                items
            }
            Err(e) => {
                log_error(&format!(
                    "[open_tray_popup] Failed to fetch tray menu: {}",
                    e
                ));
                Vec::new()
            }
        }
    } else {
        log_info(&format!(
            "[open_tray_popup] No menu_path for {}, showing empty popup",
            service_name
        ));
        Vec::new()
    };

    // Sin menú por dbusmenu, el que lo dibuja es el programa: `ContextMenu` es
    // lo que la especificación prevé para eso. Abrir el applet vacío era
    // mostrar una ficha sin nada que hacer. Si el programa tampoco lo
    // implementa, queda el applet con el aviso de que no hay acciones.
    if !has_visible_entries(&items) {
        let panel_origin = get_primary_monitor(&app)
            .map(|monitor| {
                let scale = monitor.scale_factor();
                panel_position::read().origin(
                    monitor.size().width as f64 / scale,
                    monitor.size().height as f64 / scale,
                )
            })
            .unwrap_or((0.0, 0.0));
        let (x, y) = anchor
            .as_ref()
            .map(|rect| anchor_screen_center(rect, panel_origin))
            .unwrap_or((0, 0));
        match ask_for_own_context_menu(&service_name, x, y).await {
            Ok(()) => {
                log_info(&format!(
                    "[open_tray_popup] {} no publica menú: se le pidió el suyo",
                    service_name
                ));
                return Ok(());
            }
            Err(error) => log_debug(&format!(
                "[open_tray_popup] {} tampoco atiende ContextMenu: {}",
                service_name, error
            )),
        }
    }

    let size = tray_menu_size(&items);
    let payload = SystrayPopupPayload {
        icon_id,
        icon_data,
        tooltip,
        status,
        title,
        service_name: service_name.clone(),
        items: items.clone(),
    };

    *popup_state
        .0
        .lock()
        .unwrap_or_else(|envenenado| envenenado.into_inner()) = Some(payload);

    // Se abre siempre, aunque ya estuviera abierto: el mismo applet muestra
    // el menú de otro icono, y tiene que mudarse debajo de ése. La página vuelve
    // a pedir los datos al mostrarse.
    open_anchored_applet(&app, "tray", anchor, Some(size))?;
    log_info(&format!(
        "[open_tray_popup] applet de la bandeja abierto para {}",
        service_name
    ));

    Ok(())
}

async fn ask_for_own_context_menu(service_name: &str, x: i32, y: i32) -> Result<(), String> {
    let conn = Connection::session().await.map_err(|e| e.to_string())?;
    let proxy = get_sni_proxy(&conn, service_name).await?;
    proxy.context_menu(x, y).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_tray_popup_data(
    popup_state: tauri::State<'_, SystrayPopupState>,
) -> Result<Option<SystrayPopupPayload>, String> {
    let data = popup_state
        .0
        .lock()
        .unwrap_or_else(|envenenado| envenenado.into_inner())
        .clone();
    Ok(data)
}

#[tauri::command]
pub async fn tray_popup_click(
    menu_id: i32,
    tray_manager: tauri::State<'_, TrayManager>,
    popup_state: tauri::State<'_, SystrayPopupState>,
) -> Result<(), String> {
    let service_name = {
        let data = popup_state
            .0
            .lock()
            .unwrap_or_else(|envenenado| envenenado.into_inner());
        data.as_ref().map(|d| d.service_name.clone())
    }
    .ok_or("No popup data available")?;

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
        .destination(bus_name)
        .map_err(|e| e.to_string())?
        .path(menu_path)
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;

    proxy
        .event(menu_id, "clicked", &Value::from(""), 0)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(label: &str) -> TrayMenu {
        TrayMenu {
            id: 1,
            label: label.into(),
            enabled: true,
            visible: true,
            menu_type: "standard".into(),
            checked: None,
            icon: None,
            children: None,
        }
    }

    fn separator() -> TrayMenu {
        TrayMenu {
            menu_type: "separator".into(),
            ..entry("")
        }
    }

    #[test]
    fn el_menu_mide_lo_que_ocupan_sus_entradas() {
        let items = vec![entry("Abrir"), separator(), entry("Salir")];
        assert_eq!(
            tray_menu_size(&items),
            (
                TRAY_MENU_WIDTH,
                TRAY_MENU_CHROME + 2.0 * TRAY_MENU_ITEM + TRAY_MENU_SEPARATOR
            )
        );
    }

    #[test]
    fn lo_invisible_no_ocupa_lugar() {
        let hidden = TrayMenu {
            visible: false,
            ..entry("Oculta")
        };
        assert_eq!(
            tray_menu_size(&[entry("Abrir"), hidden]),
            tray_menu_size(&[entry("Abrir")])
        );
    }

    #[test]
    fn un_submenu_suma_su_titulo_y_sus_entradas() {
        let parent = TrayMenu {
            menu_type: "submenu".into(),
            children: Some(vec![entry("Uno"), entry("Dos")]),
            ..entry("Estado")
        };
        assert_eq!(
            tray_menu_size(&[parent]).1,
            TRAY_MENU_CHROME + TRAY_MENU_CAPTION + 2.0 * TRAY_MENU_ITEM
        );
    }

    #[test]
    fn un_menu_largo_se_topa_y_desplaza() {
        let items: Vec<TrayMenu> = (0..40).map(|_| entry("x")).collect();
        assert_eq!(tray_menu_size(&items).1, TRAY_MENU_MAX_HEIGHT);
    }

    #[test]
    fn un_menu_vacio_tiene_lugar_para_el_aviso() {
        assert_eq!(tray_menu_size(&[]).1, TRAY_MENU_CHROME + TRAY_MENU_ITEM);
    }

    #[test]
    fn la_marca_del_atajo_se_va_y_el_guion_bajo_literal_queda() {
        assert_eq!(menu_label("_Salir"), "Salir");
        assert_eq!(menu_label("Abrir _carpeta"), "Abrir carpeta");
        assert_eq!(menu_label("mi__archivo"), "mi_archivo");
        assert_eq!(menu_label("Sin atajo"), "Sin atajo");
    }

    #[test]
    fn solo_se_tilda_lo_que_es_tildable() {
        assert_eq!(menu_checked(Some("checkmark"), Some(1)), Some(true));
        assert_eq!(menu_checked(Some("radio"), Some(0)), Some(false));
        assert_eq!(menu_checked(Some("checkmark"), None), Some(false));
        assert_eq!(menu_checked(None, Some(0)), None);
        assert_eq!(menu_checked(Some(""), Some(-1)), None);
    }

    #[test]
    fn el_parser_completo_conserva_el_guion_bajo_literal() {
        use std::collections::HashMap;
        let mut props: HashMap<String, Value> = HashMap::new();
        props.insert("label".into(), Value::from("mi__archivo"));
        let child = Value::from((7i32, props, Vec::<Value>::new()));
        let parsed = parse_dbus_menu_value(&child).expect("la entrada se lee");
        assert_eq!(parsed.label, "mi_archivo");
    }

    #[test]
    fn un_submenu_vacio_es_un_titulo_y_no_una_accion() {
        let empty = TrayMenu {
            menu_type: "submenu".into(),
            children: Some(vec![]),
            ..entry("Recientes")
        };
        assert!(!has_visible_entries(std::slice::from_ref(&empty)));
        assert_eq!(
            tray_menu_rows(&[empty.clone(), entry("Salir")]),
            TRAY_MENU_CAPTION + TRAY_MENU_ITEM
        );
        // Sin nada que tocar, sólo el aviso: el título suelto no se dibuja.
        assert_eq!(
            tray_menu_size(&[empty]).1,
            TRAY_MENU_CHROME + TRAY_MENU_ITEM
        );
    }

    #[test]
    fn una_entrada_dentro_de_un_submenu_cuenta_como_accion() {
        let parent = TrayMenu {
            menu_type: "submenu".into(),
            children: Some(vec![entry("Uno")]),
            ..entry("Estado")
        };
        assert!(has_visible_entries(&[parent]));
    }

    #[test]
    fn el_centro_del_boton_se_lleva_a_la_pantalla() {
        let rect = AnchorRect {
            x: 100.0,
            y: 4.0,
            width: 28.0,
            height: 28.0,
        };
        // Panel arriba: el origen es la esquina de la pantalla.
        assert_eq!(anchor_screen_center(&rect, (0.0, 0.0)), (114, 18));
        // Panel abajo en 1920×1080: empieza 36 px antes del borde.
        assert_eq!(anchor_screen_center(&rect, (0.0, 1044.0)), (114, 1062));
    }

    #[test]
    fn sin_entradas_visibles_no_hay_menu_propio() {
        assert!(!has_visible_entries(&[]));
        assert!(!has_visible_entries(&[separator()]));
        assert!(!has_visible_entries(&[TrayMenu {
            visible: false,
            ..entry("Oculta")
        }]));
        assert!(has_visible_entries(&[separator(), entry("Salir")]));
    }

    /// Reads every tray item that is actually on the bus and walks its menu with
    /// the same code the panel uses.
    ///
    /// The bug this covers does not show up without a real app on the other end:
    /// `Menu` is an object path and was declared as `String`, so zvariant rejected
    /// every read, the caller took that as "no menu" and fell back to the
    /// libayatana path — which is only right for libayatana apps. Everything else
    /// looked like an item with no context menu.
    ///
    /// Ignored by default: it needs a session bus with at least one tray app
    /// running. `cargo test -- --ignored --nocapture tray_menus`.
    #[tokio::test]
    #[ignore]
    async fn tray_menus_resolve_against_the_live_bus() {
        let conn = Connection::session().await.expect("no hay bus de sesión");

        let watcher = zbus::Proxy::new(
            &conn,
            "org.kde.StatusNotifierWatcher",
            "/StatusNotifierWatcher",
            "org.kde.StatusNotifierWatcher",
        )
        .await
        .expect("no hay StatusNotifierWatcher");

        let items: Vec<String> = watcher
            .get_property("RegisteredStatusNotifierItems")
            .await
            .expect("no se pudo leer RegisteredStatusNotifierItems");

        assert!(
            !items.is_empty(),
            "no hay items en el tray; abrí una app con icono de bandeja"
        );

        for service in &items {
            let (bus_name, object_path) = match service.split_once('/') {
                Some((bus, path)) => (bus.to_string(), format!("/{path}")),
                None => (service.clone(), "/StatusNotifierItem".to_string()),
            };

            let proxy = SniItemProxy::builder(&conn)
                .destination(bus_name.as_str())
                .unwrap()
                .path(object_path.clone())
                .unwrap()
                .build()
                .await
                .expect("no se pudo armar el proxy del item");

            // The whole point of the type fix: this used to be an Err.
            let menu_path = proxy
                .menu()
                .await
                .unwrap_or_else(|e| panic!("{service}: no se pudo leer Menu: {e}"));
            let menu_path = menu_path.as_str().to_string();
            println!("{service}\n  Menu -> {menu_path}");

            // The real path the panel takes, recursion included.
            let children = load_dbus_menu_level(&conn, &bus_name, &menu_path, 0)
                .await
                .unwrap_or_else(|e| panic!("{service}: no se pudo leer el menú: {e}"));

            println!("  {} entradas de primer nivel", children.len());
            for child in &children {
                let subs = child.children.clone().unwrap_or_default();
                println!(
                    "    id={:<4} type={:<9} sub={} {:?}",
                    child.id,
                    child.menu_type,
                    subs.len(),
                    child.label
                );
                for sub in &subs {
                    println!("        id={:<4} {:?}", sub.id, sub.label);
                }

                // A submenu that reports no children is the duplication bug's
                // other face: GetLayout used to ignore the parent id, so this
                // either came back empty or came back as a copy of the root.
                if child.menu_type == "submenu" {
                    assert!(
                        !subs.is_empty(),
                        "{service}: el submenú {:?} (id={}) quedó vacío",
                        child.label,
                        child.id
                    );
                    let duplicated = subs.len() == children.len()
                        && subs.iter().zip(children.iter()).all(|(a, b)| a.id == b.id);
                    assert!(
                        !duplicated,
                        "{service}: el submenú {:?} devolvió el menú raíz entero",
                        child.label
                    );
                }
            }

            assert!(
                !children.is_empty(),
                "{service}: el menú se leyó vacío desde {menu_path}"
            );
        }
    }
}
