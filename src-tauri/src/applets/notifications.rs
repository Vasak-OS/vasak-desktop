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
        crate::notifications::initialize_app_handle(app.clone()).await;
        crate::notifications::start_notification_server().await
    }
}
