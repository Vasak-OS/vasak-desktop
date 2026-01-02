use std::sync::Arc;
use tauri::{AppHandle, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

use crate::{app_url::get_app_url, monitor_manager::get_primary_monitor};

pub fn create_file_manager_window(
    app: AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;
    
    let label = format!("app_file_manager_{}", Uuid::new_v4());

    let mut builder = WebviewWindowBuilder::new(
            &app,
            &label,
            WebviewUrl::App("index.html#/apps/file_manager".into()),
        )
        .title("File Manager")
        .decorations(false)
        .transparent(true)
        .inner_size(800.0, 600.0)
        .min_inner_size(400.0, 300.0)
        .visible(true);


    let file_manager_window = Arc::new(builder.build()?);

    #[cfg(target_os = "linux")]
    {
        if let Ok(gtk_window) = file_manager_window.gtk_window() {
            use gtk::prelude::GtkWindowExt;
            gtk_window.set_icon_name(Some("folder"));
        }
    }

    let complete_url = format!("{}/index.html#/apps/file_manager", get_app_url());
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = file_manager_window.navigate(url);

    let monitor_size = primary_monitor.size();
    let monitor_position = primary_monitor.position();

    let center_x = monitor_position.x + (monitor_size.width as i32 / 2) - (800 / 2);
    let center_y = monitor_position.y + (monitor_size.height as i32 / 2) - (600 / 2);

    file_manager_window.set_position(Position::Physical(PhysicalPosition {
        x: center_x,
        y: center_y,
    }))?;

    file_manager_window.set_focus()?;

    Ok(())
}
