use tauri::{AppHandle, PhysicalPosition, Position, Url, WebviewUrl, WebviewWindowBuilder};


use crate::{app_url::get_app_url, monitor_manager::get_primary_monitor};

pub async fn create_menu_window(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(&app).ok_or("No primary monitor found")?;

    let menu_window =
        WebviewWindowBuilder::new(&app, "menu", WebviewUrl::App("index.html#/menu".into()))
            .title("Vasak Menu")
            .decorations(false)
            .transparent(true)
            .inner_size(900.0, 620.0)
            .max_inner_size(900.0, 620.0)
            .min_inner_size(900.0, 620.0)
            .visible(true)
            .skip_taskbar(true)
            .build()?;

    let complete_url = format!("{}/index.html#/menu", get_app_url());
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = menu_window.navigate(url);

    let monitor_size = primary_monitor.size();
    let monitor_position = primary_monitor.position();

    let center_x = monitor_position.x + (monitor_size.width as i32 / 2) - (900 / 2);
    let center_y = monitor_position.y + (monitor_size.height as i32 / 2) - (620 / 2);

    menu_window.set_position(Position::Physical(PhysicalPosition {
        x: center_x,
        y: center_y,
    }))?;

    menu_window.set_focus()?;

    Ok(())
}

