use super::Applet;
use async_trait::async_trait;
use std::error::Error;
use tauri::AppHandle;

/// Subscribes the panel to the Android device service.
///
/// Deferred on purpose: nothing here is needed to draw the panel, and most
/// sessions never have a phone plugged in. Starting it after the panel is up
/// keeps it off the path that decides how long the desktop takes to appear.
///
/// If the service is not running — it is only wanted by the graphical session,
/// and the person may have disabled it — subscribing fails quietly and the
/// panel simply never shows the phone button.
pub struct ConnectApplet;

#[async_trait]
impl Applet for ConnectApplet {
    fn name(&self) -> &'static str {
        "connect"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        log::info!("Connect applet initialized");
        crate::connect::watch_signals(app).await
    }
}
