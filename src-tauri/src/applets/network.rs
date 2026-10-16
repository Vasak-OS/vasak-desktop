use super::Applet;
use async_trait::async_trait;
use tauri::{AppHandle, Emitter};
use std::error::Error;

pub struct NetworkApplet;

#[async_trait]
impl Applet for NetworkApplet {
    fn name(&self) -> &'static str {
        "network"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        log::info!("Network applet initialized");
        
        // The network plugin is already initialized via plugin system in lib.rs
        // The plugin handles its own D-Bus signal monitoring and event emission
        // This applet just serves as a marker in the AppletManager
        
        // Emit initial status (plugin will handle actual network state)
        let _ = app.emit("network-applet-ready", ());
        
        Ok(())
    }
}
