use super::Applet;
use async_trait::async_trait;
use tauri::{AppHandle, Emitter};
use std::error::Error;

use crate::commands::osd::show_osd_internal;

pub struct BrightnessApplet;

#[async_trait]
impl Applet for BrightnessApplet {
    fn name(&self) -> &'static str {
        "brightness"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        log::info!("Brightness applet starting monitoring");
        monitor_brightness(app);
        Ok(())
    }
}

fn monitor_brightness(app: AppHandle) {
    tokio::spawn(async move {
        let device_path = match find_backlight_device() {
            Some(p) => p,
            None => {
                log::warn!("No backlight device found. Brightness monitoring disabled.");
                return;
            }
        };

        let brightness_path = device_path.join("actual_brightness");
        let max_path = device_path.join("max_brightness");

        let mut interval_ms = 2000;
        let mut no_change_count = 0;
        let mut last_val = -1;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await;

            let current_res = read_int_file(&brightness_path).await;
            let max_res = read_int_file(&max_path).await;

            if let (Ok(current), Ok(max)) = (current_res, max_res) {
                 if current != last_val {
                     last_val = current;

                     interval_ms = 200;
                     no_change_count = 0;

                     let percentage = if max > 0 {
                         (current as f64 / max as f64 * 100.0).round() as u32
                     } else {
                         0
                     };

                     let _ = app.emit("brightness-changed", serde_json::json!({
                         "current": percentage,
                         "max": 100,
                         "min": 0
                     }));

                     let _ = show_osd_internal(
                         "display-brightness",
                         percentage as f64,
                         100.0,
                         &format!("Brillo: {}%", percentage),
                         &app,
                     )
                     .await;
                 } else {
                     no_change_count += 1;
                     if no_change_count > 10 {
                         interval_ms = 2000;
                     }
                 }
            } else {
                log::error!("Failed to read brightness values");
                interval_ms = 5000;
            }
        }
    });
}

fn find_backlight_device() -> Option<std::path::PathBuf> {
    let base = std::path::Path::new("/sys/class/backlight");
    if !base.exists() {
        return None;
    }

    if let Ok(entries) = std::fs::read_dir(base) {
        if let Some(entry) = entries.flatten().next() {
            return Some(entry.path());
        }
    }
    None
}

async fn read_int_file(path: &std::path::Path) -> Result<i32, std::io::Error> {
    let content = tokio::fs::read_to_string(path).await?;
    content.trim().parse::<i32>().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
