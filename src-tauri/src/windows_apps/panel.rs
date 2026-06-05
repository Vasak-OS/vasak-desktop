use gdk::prelude::*;
use gtk::prelude::*;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use tauri::{
    App, WebviewUrl, WebviewWindowBuilder,
};
use crate::monitor_manager::get_primary_monitor;

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

pub fn create_panel(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(app.handle()).ok_or("No primary monitor found")?;
    let monitor_size = primary_monitor.size();
    let width = monitor_size.width as i32;

    let layer_monitor = find_gdk_monitor(&primary_monitor).ok_or("No matching GDK monitor found for primary monitor")?;

    // 1. Crear WebviewWindow de Tauri (xdg-toplevel interno) para tener IPC listo.
    let panel_window = WebviewWindowBuilder::new(
        app,
        "panel",
        WebviewUrl::App("index.html#/panel".into()),
    )
    .title("Vasak Panel")
    .decorations(false)
    .inner_size(monitor_size.width as f64, 38.0)
    .visible(false)
    .build()?;

    let gtk_window = panel_window.gtk_window()?;

    // 2. Crear GtkWindow con layer-shell para el panel real.
    let layer_win = gtk::Window::new(gtk::WindowType::Toplevel);
    layer_win.set_decorated(false);
    layer_win.set_size_request(width, 38);

    // Configurar layer-shell: Top, exclusive zone automático, sin teclado
    layer_win.init_layer_shell();
    layer_win.set_monitor(&layer_monitor);
    layer_win.set_namespace("vasak-panel");
    layer_win.set_layer(Layer::Top);
    layer_win.set_anchor(Edge::Top, true);
    layer_win.set_anchor(Edge::Left, true);
    layer_win.set_anchor(Edge::Right, true);
    layer_win.auto_exclusive_zone_enable();
    layer_win.set_keyboard_mode(KeyboardMode::None);

    // 3. Reparent: extraer el WebKitWebView de la ventana xdg y ponerlo en layer-shell.
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
            "No se pudo reparentar el webview: child={child_type}, container={container_type}"
        ).into());
    }

    // 4. Forzar fondo transparente en la ventana layer-shell.
    if let Some(screen) = gtk::prelude::WidgetExt::screen(&layer_win) {
        if let Some(rgba) = screen.rgba_visual() {
            layer_win.set_visual(Some(&rgba));
        }
        let css = gtk::CssProvider::new();
        css.load_from_data(
            b"window { background-color: rgba(0, 0, 0, 0); }"
        ).ok();
        layer_win.style_context().add_provider(
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
        );
    }
    let _ = panel_window.set_background_color(Some(tauri::webview::Color(0, 0, 0, 0)));

    // 5. Mostrar layer-shell, ocultar la original xdg
    layer_win.show_all();
    gtk_window.hide();

    std::mem::forget(layer_win);

    Ok(())
}
