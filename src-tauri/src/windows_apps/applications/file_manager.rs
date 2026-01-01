use std::sync::Arc;
use tauri::{AppHandle, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

use crate::{app_url::get_app_url, monitor_manager::get_primary_monitor};

pub async fn create_file_manager_window(
    app: AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;
    
    // Generate a unique label for this instance
    let label = format!("app_file_manager_{}", Uuid::new_v4());

    let file_manager_window = Arc::new(
        WebviewWindowBuilder::new(
            &app,
            &label,
            WebviewUrl::App("index.html#/apps/file_manager".into()),
        )
        .title("File Manager")
        .decorations(false)
        .transparent(true)
        .inner_size(800.0, 600.0) // Default size
        .min_inner_size(400.0, 300.0)
        .visible(true)
        .build()?,
    );

    let complete_url = format!("{}/index.html#/apps/file_manager", get_app_url());
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = file_manager_window.navigate(url);

    let monitor_size = primary_monitor.size();
    let monitor_position = primary_monitor.position();

    // Center the window initially
    let center_x = monitor_position.x + (monitor_size.width as i32 / 2) - (800 / 2);
    let center_y = monitor_position.y + (monitor_size.height as i32 / 2) - (600 / 2);

    // Add a slight offset if there are multiple windows (optional, but good for UX)
    // For now simple centering is fine, the OS or window manager usually handles stacking offset if we don't force it hard every time.
    // However, since we are manually positioning, all will stack perfectly on top. 
    // Let's add a small random offset to see them separate? 
    // Or just let them stack. Stacking is standard behavior.

    file_manager_window.set_position(Position::Physical(PhysicalPosition {
        x: center_x,
        y: center_y,
    }))?;

    file_manager_window.set_focus()?;

    Ok(())
}
