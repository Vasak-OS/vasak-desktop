use super::Applet;
use async_trait::async_trait;
use tauri::AppHandle;
use std::error::Error;

pub struct BrightnessApplet;

#[async_trait]
impl Applet for BrightnessApplet {
    fn name(&self) -> &'static str {
        "brightness"
    }

    async fn start(&self, _app: AppHandle) -> Result<(), Box<dyn Error>> {
        // Brightness applet doesn't need active monitoring currently
        // It only provides commands (get_brightness, set_brightness)
        // This is here for consistency and future expansion (e.g., monitoring hardware brightness changes)
        log::info!("Brightness applet initialized (command-only mode)");
        Ok(())
    }
}
