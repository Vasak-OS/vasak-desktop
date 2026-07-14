use gdk::prelude::*;
use gtk::prelude::*;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use tauri::{App, WebviewUrl, WebviewWindowBuilder};

use crate::gtk_utils;
use crate::monitor_manager::{get_monitors, get_primary_monitor};

pub fn create_desktops(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let monitors = get_monitors(app.handle()).ok_or("No monitors found")?;
    let primary_monitor = get_primary_monitor(app.handle()).ok_or("No primary monitor found")?;

    setup_desktop(app, "desktop", &primary_monitor, false)?;
    open_other_desktops(app, &monitors, &primary_monitor);

    Ok(())
}

fn open_other_desktops(app: &App, monitors: &[tauri::Monitor], primary_monitor: &tauri::Monitor) {
    let primary_pos = primary_monitor.position();
    let app_handle = app.handle().clone();
    let others: Vec<tauri::Monitor> = monitors
        .iter()
        .filter(|m| m.position() != primary_pos)
        .cloned()
        .collect();

    for (index, monitor) in others.into_iter().enumerate() {
        let label = format!("desktop_{}", index + 1);
        let app = app_handle.clone();
        std::thread::spawn(move || {
            // We create the webview window from the main thread via invoke_on_main,
            // but we need the AppHandle. Since this is a secondary monitor and
            // the setup is complex, we handle it inside invoke_on_main
            unsafe {
                gtk_utils::invoke_on_main(move || {
                    unimplemented!("Secondary monitor desktops not yet supported");
                });
            }
        });
    }
}

fn setup_desktop(
    app: &App,
    label: &str,
    tauri_monitor: &tauri::Monitor,
    _is_secondary: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let pos = tauri_monitor.position();
    let size = tauri_monitor.size();

    // 1. Find matching GDK monitor.
    let gdk_monitor = find_gdk_monitor(tauri_monitor)
        .ok_or_else(|| format!("No GDK monitor for {:?}", label))?;

    // 2. Create Tauri WebviewWindow (xdg-toplevel) to host the webview.
    let desktop_window = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::App(format!("index.html#/desktop?monitor={}", label).into()),
    )
    .title(format!("Vasak Desktop {}", label))
    .decorations(false)
    .transparent(true)
    .inner_size(size.width as f64, size.height as f64)
    .position(pos.x as f64, pos.y as f64)
    .visible(false)
    .skip_taskbar(true)
    .build()?;

    let gtk_window = desktop_window.gtk_window()?;

    // 3. Create a fresh GtkWindow with layer-shell at Background layer.
    let layer_win = gtk::Window::new(gtk::WindowType::Toplevel);
    layer_win.set_decorated(false);
    layer_win.set_size_request(size.width as i32, size.height as i32);
    layer_win.set_position(gtk::WindowPosition::None);
    layer_win.move_(pos.x, pos.y);

    layer_win.init_layer_shell();
    layer_win.set_monitor(&gdk_monitor);
    layer_win.set_namespace("vasak-desktop");
    layer_win.set_layer(Layer::Background);
    layer_win.set_anchor(Edge::Left, true);
    layer_win.set_anchor(Edge::Right, true);
    layer_win.set_anchor(Edge::Top, true);
    layer_win.set_anchor(Edge::Bottom, true);
    layer_win.set_keyboard_mode(KeyboardMode::None);

    // 4. Reparent: extract webview from Tauri's xdg window into layer-shell window.
    let (reparented, child_type, container_type) = gtk_window.child().map_or(
        (false, "None".to_string(), "None".to_string()),
        |vbox| {
            let child_name = vbox.type_().name().to_string();
            let container = vbox.dynamic_cast_ref::<gtk::Container>();
            match container {
                Some(container) => {
                    let container_name = container.type_().name().to_string();
                    match container.children().first() {
                        Some(widget) => {
                            let widget_name = widget.type_().name().to_string();
                            container.remove(widget);
                            layer_win.add(widget);
                            (true, widget_name, container_name)
                        }
                        None => (false, child_name, container_name),
                    }
                }
                None => (false, child_name, "Not a Container".to_string()),
            }
        },
    );

    if !reparented {
        return Err(format!(
            "Desktop reparent failed: child={child_type}, container={container_type}"
        ).into());
    }

    // 5. Transparent background.
    if let Some(screen) = gtk::prelude::WidgetExt::screen(&layer_win) {
        if let Some(rgba) = screen.rgba_visual() {
            layer_win.set_visual(Some(&rgba));
        }
        let css = gtk::CssProvider::new();
        css.load_from_data(
            b"window { background-color: rgba(0, 0, 0, 0); }",
        ).ok();
        layer_win.style_context().add_provider(
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
        );
    }

    // 6. Show layer-shell desktop, hide xdg window.
    layer_win.show_all();
    gtk_window.hide();

    std::mem::forget(layer_win);

    Ok(())
}

fn find_gdk_monitor(tauri_monitor: &tauri::Monitor) -> Option<gdk::Monitor> {
    let pos = tauri_monitor.position();
    let display = gdk::Display::default()?;
    for i in 0..display.n_monitors() {
        let mon = display.monitor(i)?;
        let rect = mon.geometry();
        if rect.x() == pos.x && rect.y() == pos.y {
            return Some(mon);
        }
    }
    None
}
