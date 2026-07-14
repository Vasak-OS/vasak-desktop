use super::Applet;
use async_trait::async_trait;
use tauri::{AppHandle, Emitter};
use std::error::Error;
use std::path::PathBuf;

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
        // Find device path once
        let device_path = match find_backlight_device() {
            Some(p) => p,
            None => {
                log::warn!("No backlight device found under /sys/class/backlight. Brightness monitoring disabled.");
                return;
            }
        };

        let brightness_path = device_path.join("actual_brightness");
        let max_path = device_path.join("max_brightness");

        // Attempt inotify-based monitoring first
        if try_inotify_monitor(app.clone(), brightness_path.clone(), max_path.clone()).await {
            return; // inotify is running successfully
        }

        // Fallback to adaptive polling
        log::warn!("inotify monitoring failed, falling back to adaptive polling for brightness");
        adaptive_poll_monitor(app, brightness_path, max_path).await;
    });
}

/// Attempts to set up inotify-based monitoring on the brightness sysfs file.
/// Returns `true` if inotify is running (the function will continue running until error),
/// or `false` if inotify setup failed and we should fall back to polling.
async fn try_inotify_monitor(app: AppHandle, brightness_path: PathBuf, max_path: PathBuf) -> bool {
    use inotify::{Inotify, WatchMask};
    use futures_util::StreamExt;

    // Attempt inotify init
    let inotify = match Inotify::init() {
        Ok(i) => i,
        Err(e) => {
            log::warn!("Failed to initialize inotify: {}", e);
            return false;
        }
    };

    // Attempt to add watch on brightness file
    if let Err(e) = inotify.watches().add(&brightness_path, WatchMask::MODIFY) {
        log::warn!("Failed to add inotify watch on {:?}: {}", brightness_path, e);
        return false;
    }

    log::info!("inotify watch established on {:?}", brightness_path);

    // Create the async event stream
    let buffer = [0; 1024];
    let mut stream = inotify.into_event_stream(buffer).expect("Failed to create inotify event stream");

    // Perform an initial read to check if the file is accessible, and to validate inotify is working.
    // Some sysfs backlight drivers don't trigger inotify events properly.
    // We'll do a validation: wait up to 5 seconds. If no event arrives but the value changes
    // (detected via a single comparison read), inotify is unreliable and we should fall back.
    let initial_value = match read_int_file(&brightness_path).await {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Failed initial brightness read: {}. Falling back to polling.", e);
            return false;
        }
    };

    // Emit initial brightness
    if let Ok(max) = read_int_file(&max_path).await {
        emit_brightness(&app, initial_value, max);
    }

    // Validate inotify works: wait for first event with a 5-second timeout.
    // If no event arrives but value has changed, inotify is broken for this driver.
    let validation_timeout = tokio::time::Duration::from_secs(5);
    let first_event = tokio::time::timeout(validation_timeout, stream.next()).await;

    match first_event {
        Ok(Some(Ok(_event))) => {
            // inotify is working! Process this event and continue the loop
            if let Ok(current) = read_int_file(&brightness_path).await {
                if let Ok(max) = read_int_file(&max_path).await {
                    emit_brightness(&app, current, max);
                }
            }
        }
        Ok(Some(Err(e))) => {
            log::warn!("inotify stream error during validation: {}. Falling back.", e);
            return false;
        }
        Ok(None) => {
            log::warn!("inotify stream ended unexpectedly. Falling back.");
            return false;
        }
        Err(_timeout) => {
            // Timeout - check if value actually changed (would mean inotify missed it)
            if let Ok(current) = read_int_file(&brightness_path).await {
                if current != initial_value {
                    log::warn!(
                        "Brightness value changed ({} -> {}) but no inotify event received. \
                         This backlight driver doesn't support inotify. Falling back to polling.",
                        initial_value, current
                    );
                    return false;
                }
            }
            // No change and no event - inotify might still work, just no brightness change happened.
            // Continue with inotify loop.
            log::info!("inotify validation passed (no changes detected in 5s window, inotify assumed functional).");
        }
    }

    // Main inotify event loop - read brightness exactly once per MODIFY event
    let mut last_value = initial_value;
    loop {
        match stream.next().await {
            Some(Ok(_event)) => {
                // Read exactly once per inotify event
                match read_int_file(&brightness_path).await {
                    Ok(current) => {
                        if current != last_value {
                            last_value = current;
                            if let Ok(max) = read_int_file(&max_path).await {
                                emit_brightness(&app, current, max);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read brightness after inotify event: {}", e);
                        // Keep last known value and continue waiting for next event
                    }
                }
            }
            Some(Err(e)) => {
                log::error!("inotify stream error: {}. Stopping inotify monitor.", e);
                break;
            }
            None => {
                log::warn!("inotify stream ended. Stopping inotify monitor.");
                break;
            }
        }
    }

    // inotify failed mid-operation, fall back to polling
    log::warn!("inotify monitor stopped, switching to adaptive polling fallback");
    adaptive_poll_monitor(app, brightness_path, max_path).await;
    true // We handled everything (including fallback)
}

/// Adaptive polling fallback: 2000ms slow, 200ms fast when changes detected,
/// 5000ms on read failure until recovery.
async fn adaptive_poll_monitor(app: AppHandle, brightness_path: PathBuf, max_path: PathBuf) {
    let mut interval_ms: u64 = 2000;
    let mut no_change_count: u32 = 0;
    let mut last_val: i32 = -1;

    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await;

        let current_res = read_int_file(&brightness_path).await;
        let max_res = read_int_file(&max_path).await;

        if let (Ok(current), Ok(max)) = (current_res, max_res) {
            if current != last_val {
                // Value changed - switch to fast polling
                last_val = current;
                interval_ms = 200;
                no_change_count = 0;

                emit_brightness(&app, current, max);
            } else {
                // No change
                no_change_count += 1;
                if no_change_count > 10 {
                    // ~2 seconds of stability at 200ms, return to slow polling
                    interval_ms = 2000;
                }
            }
        } else {
            log::error!("Failed to read brightness values");
            // Retain last known value, increase interval to 5000ms until recovery
            interval_ms = 5000;
        }
    }
}

/// Emit a brightness-changed event to the frontend.
fn emit_brightness(app: &AppHandle, current: i32, max: i32) {
    let percentage = if max > 0 {
        (current as f64 / max as f64 * 100.0).round() as u8
    } else {
        0
    };

    let _ = app.emit(
        "brightness-changed",
        serde_json::json!({
            "current": percentage,
            "max": 100,
            "min": 0
        }),
    );
}

fn find_backlight_device() -> Option<PathBuf> {
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
    content
        .trim()
        .parse::<i32>()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
