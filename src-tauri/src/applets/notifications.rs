use super::Applet;
use async_trait::async_trait;
use tauri::AppHandle;
use std::error::Error;

pub struct NotificationApplet;

#[async_trait]
impl Applet for NotificationApplet {
    fn name(&self) -> &'static str {
        "notifications"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        // The freedesktop server now lives in vasak-flare-daemon; here we only
        // start the client (reads history, follows the daemon's Changed signal).
        crate::notifications::initialize_app_handle(app).await;
        Ok(())
    }
}
