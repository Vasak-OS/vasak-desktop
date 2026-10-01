use serde::Deserialize;
use zbus::zvariant::{Type, OwnedValue};
use zbus::proxy;

/// Cuánto se espera a `AboutToShow`, que es opcional.
const ABOUT_TO_SHOW_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);
/// Cuánto se espera a `GetLayout` antes de dar el menú por perdido.
const GET_LAYOUT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// Estructura que representa un nodo del menú DBusMenu
#[derive(Debug, Deserialize, Type)]
pub struct DbusMenuLayout(
    pub i32,                          // id
    pub std::collections::HashMap<String, OwnedValue>, // properties
    pub Vec<OwnedValue>, // children (recursive variant)
);

/// Get the D-Bus menu layout by manually calling GetLayout.
/// The response has TWO top-level body items (UINT32 + struct), but zbus
/// proxy tuple expects them wrapped in a D-Bus struct. We call manually.
///
/// `parent_id` is the node to read. It used to be hardcoded to 0, so asking for a
/// submenu's contents returned the root again and the caller filed the whole menu
/// away as that submenu's children — every submenu showed a copy of the entire
/// menu. Servers commonly leave a submenu's children out of the root reply and
/// only fill them in when asked for that node directly, which is why the depth
/// below is not enough on its own.
pub async fn call_get_layout(
    conn: &zbus::Connection,
    bus_name: &str,
    menu_path: &str,
    parent_id: i32,
) -> zbus::Result<(i32, DbusMenuLayout)> {
    let args = (parent_id, -1i32, Vec::<&str>::new());
    let call = conn.call_method(
        Some(bus_name),
        menu_path,
        Some("com.canonical.dbusmenu"),
        "GetLayout",
        &args,
    );
    // zbus 4 no pone tope a una llamada: un programa colgado que no contesta
    // dejaba el clic derecho esperando para siempre.
    let reply = tokio::time::timeout(GET_LAYOUT_TIMEOUT, call)
        .await
        .map_err(|_| zbus::Error::Failure("GetLayout no contestó a tiempo".into()))??;

    let body = reply.body();
    let data = body.data();

    // First body item: revision as UINT32
    let (revision, consumed): (u32, _) = data
        .deserialize_for_signature("u")
        .map_err(|e| zbus::Error::Failure(format!("Failed to deserialize revision: {e}")))?;

    // Second body item: layout struct (ia{sv}av)
    let layout_data = data.slice(consumed..);
    let (layout, _): (DbusMenuLayout, _) = layout_data
        .deserialize_for_signature("(ia{sv}av)")
        .map_err(|e| zbus::Error::Failure(format!("Failed to deserialize layout: {e}")))?;

    Ok((revision as i32, layout))
}

/// Call AboutToShow on the D-Bus menu (optional notification, ignore failures).
pub async fn call_about_to_show(
    conn: &zbus::Connection,
    bus_name: &str,
    menu_path: &str,
    id: i32,
) {
    let call = conn.call_method(
        Some(bus_name),
        menu_path,
        Some("com.canonical.dbusmenu"),
        "AboutToShow",
        &id,
    );
    // Es un aviso: si no contesta enseguida, el menú se pide igual.
    let _ = tokio::time::timeout(ABOUT_TO_SHOW_TIMEOUT, call).await;
}

#[proxy(
    interface = "com.canonical.dbusmenu",
)]
trait DbusMenu {
    /// Event method
    fn event(&self, id: i32, event_id: &str, data: &zvariant::Value<'_>, timestamp: u32) -> zbus::Result<()>;

    /// AboutToShow method
    fn about_to_show(&self, id: i32) -> zbus::Result<bool>;
}
