use super::Applet;
use async_trait::async_trait;
use tauri::AppHandle;
use std::error::Error;

pub struct AudioApplet;

#[async_trait]
impl Applet for AudioApplet {
    fn name(&self) -> &'static str {
        "audio"
    }

    async fn start(&self, _app: AppHandle) -> Result<(), Box<dyn Error>> {
        // Audio applet doesn't need active monitoring currently
        // It only provides commands (get_volume, set_volume, toggle_mute)
        // This is here for consistency and future expansion (e.g., monitoring external volume changes)
        log::info!("Audio applet initialized (command-only mode)");
        Ok(())
    }
}
