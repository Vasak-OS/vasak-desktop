use tauri::{AppHandle, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder};

use crate::{app_url::get_app_url, monitor_manager::get_primary_monitor};

/// The window listing a phone's applications.
///
/// Deliberately narrower and shorter than the application menu: it holds one
/// list, not a grid of categories, and a phone's app list is read by scrolling.
pub async fn create_connect_window(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;

    const WIDTH: f64 = 520.0;
    const HEIGHT: f64 = 640.0;

    let window =
        WebviewWindowBuilder::new(&app, "connect", WebviewUrl::App("index.html#/connect".into()))
            .title("Aplicaciones del teléfono")
            .decorations(false)
            .transparent(true)
            .inner_size(WIDTH, HEIGHT)
            .max_inner_size(WIDTH, HEIGHT)
            .min_inner_size(WIDTH, HEIGHT)
            .visible(true)
            .skip_taskbar(true)
            .build()?;

    let complete_url = format!("{}/index.html#/connect", get_app_url());
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = window.navigate(url);

    let monitor_size = primary_monitor.size();
    let monitor_position = primary_monitor.position();

    let center_x = monitor_position.x + (monitor_size.width as i32 / 2) - (WIDTH as i32 / 2);
    let center_y = monitor_position.y + (monitor_size.height as i32 / 2) - (HEIGHT as i32 / 2);

    window.set_position(Position::Physical(PhysicalPosition {
        x: center_x,
        y: center_y,
    }))?;

    window.set_focus()?;

    Ok(())
}
