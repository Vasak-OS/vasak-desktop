use gtk::prelude::*;
use gtk_layer_shell::{Edge, Layer, LayerShell};
use tauri::{
    App, WebviewUrl, WebviewWindowBuilder,
};
use crate::monitor_manager::get_primary_monitor;

pub fn create_panel(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(app.handle()).ok_or("No primary monitor found")?;
    let monitor_size = primary_monitor.size();
    let width = monitor_size.width as i32;

    // 1. Crear WebviewWindow de Tauri (xdg-toplevel interno) para tener IPC listo.
    let panel_window = WebviewWindowBuilder::new(
        app,
        "panel",
        WebviewUrl::App("index.html#/panel".into()),
    )
    .title("Vasak Panel")
    .decorations(false)
    .transparent(true)
    .inner_size(monitor_size.width as f64, 38.0)
    .visible(false)
    .build()?;

    let gtk_window = panel_window.gtk_window()?;

    // 2. Crear GtkWindow con layer-shell para el panel real.
    let layer_win = gtk::Window::new(gtk::WindowType::Toplevel);
    layer_win.set_decorated(false);
    layer_win.set_size_request(width, 38);

    // Visual RGBA (si hay soporte)
    if let Some(screen) = gtk::prelude::WidgetExt::screen(&layer_win) {
        if let Some(rgba) = screen.rgba_visual() {
            layer_win.set_visual(Some(&rgba));
        }
    }

    // Configurar layer-shell: Top, exclusive_zone(38), keyboard interactivo
    layer_win.init_layer_shell();
    layer_win.set_namespace("vasak-panel");
    layer_win.set_layer(Layer::Top);
    layer_win.set_anchor(Edge::Top, true);
    layer_win.set_anchor(Edge::Left, true);
    layer_win.set_anchor(Edge::Right, true);
    layer_win.set_exclusive_zone(38);
    layer_win.set_keyboard_interactivity(false);

    // 3. Reparent: extraer el WebKitWebView de la ventana xdg y ponerlo en layer-shell.
    // No usamos with_webview porque PlatformWebview no es Send.
    // La jerarquía típica de Tauri/wry en Linux es:
    //   GtkApplicationWindow > GtkBox(Vertical) > WebKitWebView
    if let Some(vbox) = gtk_window.child() {
        if let Some(container) = vbox.dynamic_cast_ref::<gtk::Container>() {
            let children = container.children();
            if let Some(widget) = children.first() {
                let widget = widget.clone();
                container.remove(&widget);
                layer_win.add(&widget);
            }
        }
    }

    // 4. Fondo transparente via CSS (si funciona con layer-shell, bien; si no, opaco)
    if let Some(screen) = gtk::prelude::WidgetExt::screen(&gtk_window) {
        let css = gtk::CssProvider::new();
        css.load_from_data(b"window { background: transparent; }").ok();
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    let _ = panel_window.set_background_color(Some(tauri::webview::Color(0, 0, 0, 0)));

    // 5. Mostrar layer-shell, ocultar la original xdg
    layer_win.show_all();
    gtk_window.hide();

    std::mem::forget(layer_win);

    Ok(())
}
