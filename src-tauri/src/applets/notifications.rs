use super::Applet;
use async_trait::async_trait;
use std::error::Error;
use tauri::AppHandle;

pub struct NotificationApplet;

#[async_trait]
impl Applet for NotificationApplet {
    fn name(&self) -> &'static str {
        "notifications"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        // The freedesktop server now lives in vasak-flare-daemon; here we only
        // start the client (reads history, follows the daemon's Changed signal).
        crate::notifications::initialize_app_handle(app.clone()).await;
        // «No molestar» lo guarda el mismo demonio; esto sólo lo sigue.
        crate::do_not_disturb::start(app).await;
        Ok(())
    }
}
