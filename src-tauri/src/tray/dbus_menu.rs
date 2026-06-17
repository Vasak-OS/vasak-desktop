use serde::Deserialize;
use zbus::zvariant::{Type, OwnedValue};
use zbus::proxy;

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
pub async fn call_get_layout(
    conn: &zbus::Connection,
    bus_name: &str,
    menu_path: &str,
) -> zbus::Result<(i32, DbusMenuLayout)> {
    let reply = conn.call_method(
        Some(bus_name),
        menu_path,
        Some("com.canonical.dbusmenu"),
        "GetLayout",
        &(0i32, -1i32, Vec::<&str>::new()),
    ).await?;

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
    let _ = conn.call_method(
        Some(bus_name),
        menu_path,
        Some("com.canonical.dbusmenu"),
        "AboutToShow",
        &id,
    ).await;
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
