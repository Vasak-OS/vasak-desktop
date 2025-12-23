use super::Applet;
use crate::structs::TrayManager;
use crate::tray::sni_watcher::SniWatcher;
use async_trait::async_trait;
use tauri::{AppHandle, Manager};

pub struct TrayApplet;

#[async_trait]
impl Applet for TrayApplet {
    fn name(&self) -> &'static str {
        "tray"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        let tray_manager = app.state::<TrayManager>();
        
        let watcher = SniWatcher::new(
            (*tray_manager).clone(),
            app.clone()
        ).await?;
        
        watcher.start_watching().await?;
        
        Ok(())
    }
}
