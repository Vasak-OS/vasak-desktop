use super::Applet;
use async_trait::async_trait;
use tauri::{AppHandle, Emitter};
use std::error::Error;

pub struct BluetoothApplet;

#[async_trait]
impl Applet for BluetoothApplet {
    fn name(&self) -> &'static str {
        "bluetooth"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        log::info!("Bluetooth applet initialized");
        
        // The bluetooth plugin is already initialized via plugin system in lib.rs
        // The plugin handles its own D-Bus signal monitoring and event emission
        // This applet just serves as a marker in the AppletManager
        
        // Emit initial status (plugin will handle actual bluetooth state)
        let _ = app.emit("bluetooth-applet-ready", ());
        
        Ok(())
    }
}
