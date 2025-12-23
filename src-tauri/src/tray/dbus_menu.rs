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


#[proxy(
    interface = "com.canonical.dbusmenu",
)]
trait DbusMenu {
    /// GetLayout method
    /// parentId: i32
    /// recursionDepth: i32 (-1 for infinite)
    /// propertyNames: Vec<&str>
    fn get_layout(&self, parent_id: i32, recursion_depth: i32, property_names: &[&str]) -> zbus::Result<(u32, DbusMenuLayout)>;

    /// Event method
    fn event(&self, id: i32, event_id: &str, data: &zvariant::Value<'_>, timestamp: u32) -> zbus::Result<()>;
}
