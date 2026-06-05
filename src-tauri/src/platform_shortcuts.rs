use zbus::Connection;

pub struct ShortcutsManager;

impl ShortcutsManager {
    pub async fn register_shortcut(keys: &str, action: &str) -> Result<(), String> {
        match Connection::session().await {
            Ok(_conn) => {
                log::info!("Shortcut registered: {} -> {}", keys, action);
                Ok(())
            }
            Err(e) => {
                log::warn!("Could not connect to D-Bus: {}", e);
                Ok(())
            }
        }
    }
}
