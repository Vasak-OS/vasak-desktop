mod commands;
mod eventloops;
mod structs;
mod tray;
mod window_manager;
mod window_properties;

use commands::{get_windows, toggle_window};
use eventloops::setup_event_monitoring;
use std::sync::{Arc, Mutex};
use structs::WMState;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_config_manager;
use tray::create_tray_manager;
use window_manager::WindowManager;
use window_properties::{set_desktop_window_properties, set_dock_window_properties};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize Windows Manager
    let window_manager = Arc::new(Mutex::new(
        WindowManager::new().expect("Failed to initialize window manager"),
    ));

    let wm_state = WMState {
        window_manager: window_manager.clone(),
    };

    let tray_manager = create_tray_manager();

    tauri::Builder::default()
        .manage(wm_state)
        .manage(tray_manager)
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("panel") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_vicons::init())
        .invoke_handler(tauri::generate_handler![get_windows, toggle_window,])
        .setup(move |app| {
            // Crear una ventana temporal para acceder a los monitores
            let temp_window =
                WebviewWindowBuilder::new(app, "temp", WebviewUrl::App("index.html".into()))
                    .title("Temp")
                    .inner_size(1.0, 1.0)
                    .visible(false)
                    .build()?;

            let available_monitors = temp_window.available_monitors()?;

            let primary_monitor = available_monitors
                .iter()
                .find(|monitor| monitor.position().x == 0 && monitor.position().y == 0)
                .unwrap_or(&available_monitors[0]); // Si no se encuentra, usamos el primer monitor por defecto.

            let primary_monitor_size = primary_monitor.size();
            let primary_monitor_position = primary_monitor.position();

            // Cerrar la ventana temporal
            temp_window.close()?;

            // Crear la ventana del escritorio para cada monitor
            for (index, monitor) in available_monitors.iter().enumerate() {
                let desktop_window = WebviewWindowBuilder::new(
                    app,
                    &format!("desktop_{}", index),
                    WebviewUrl::App(format!("index.html#/desktop?monitor={}", index).into()),
                )
                .title(&format!("Vasak Desktop {}", index))
                .decorations(false)
                .position(monitor.position().x as f64, monitor.position().y as f64)
                .inner_size(monitor.size().width as f64, monitor.size().height as f64)
                .build()?;

                set_desktop_window_properties(&desktop_window);
            }

            // Crear la ventana del panel solo en el monitor primario
            let panel_window = WebviewWindowBuilder::new(
                app,
                "panel",
                WebviewUrl::App("index.html#/panel".into()),
            )
            .title("Vasak Panel")
            .decorations(false)
            .always_on_top(true)
            .resizable(false)
            .skip_taskbar(true)
            .position(
                primary_monitor_position.x as f64,
                primary_monitor_position.y as f64,
            )
            .inner_size(primary_monitor_size.width as f64, 32.0)
            .build()?;

            set_dock_window_properties(&panel_window);

            setup_event_monitoring(window_manager.clone(), app.handle().clone())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
