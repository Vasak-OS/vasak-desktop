use gtk::prelude::*;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::collections::HashMap;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

// Layer-shell windows owned by the shell, keyed by window label.
//
// These used to be handed to `std::mem::forget` so GTK would keep them alive.
// That works until a monitor is unplugged: without a handle there is no way to
// take the window down, so a stale panel or wallpaper lingers on an output that
// no longer exists. GTK objects are not `Send`, so the registry is thread-local
// on the main thread, which is the only place these are ever touched.
thread_local! {
    static LAYER_WINDOWS: RefCell<HashMap<String, gtk::Window>> = RefCell::new(HashMap::new());
}

/// Where the layer-shell window sits and how it reserves space.
pub struct LayerSpec {
    pub namespace: &'static str,
    pub layer: Layer,
    /// Anchored edges: (left, right, top, bottom).
    pub anchors: (bool, bool, bool, bool),
    /// `None` reserves space automatically (panel); `Some(-1)` opts out entirely
    /// so other surfaces can overlap (wallpaper).
    pub exclusive_zone: Option<i32>,
    pub height_request: Option<i32>,
}

/// Builds a Tauri webview, moves it into a layer-shell window pinned to
/// `gdk_monitor`, and registers it so it can be torn down later.
///
/// Tauri can only create xdg-toplevels, which a compositor is free to place
/// wherever it likes; a shell needs its surfaces anchored to a specific output.
/// So the webview is built inside a throwaway toplevel and reparented into a
/// layer-shell window, and the now-empty toplevel is hidden.
pub fn spawn_layer_window(
    app: &AppHandle,
    label: &str,
    route: &str,
    gdk_monitor: &gdk::Monitor,
    size: (f64, f64),
    spec: LayerSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    let (width, height) = size;

    let webview = WebviewWindowBuilder::new(app, label, WebviewUrl::App(route.into()))
        .title(format!("Vasak {}", label))
        .decorations(false)
        .transparent(true)
        .inner_size(width, height)
        .visible(false)
        .skip_taskbar(true)
        .build()?;

    let gtk_window = webview.gtk_window()?;

    let layer_win = gtk::Window::new(gtk::WindowType::Toplevel);
    layer_win.set_decorated(false);

    if let Some(height) = spec.height_request {
        layer_win.set_size_request(width as i32, height);
    }

    layer_win.init_layer_shell();
    layer_win.set_monitor(gdk_monitor);
    layer_win.set_namespace(spec.namespace);
    layer_win.set_layer(spec.layer);

    let (left, right, top, bottom) = spec.anchors;
    layer_win.set_anchor(Edge::Left, left);
    layer_win.set_anchor(Edge::Right, right);
    layer_win.set_anchor(Edge::Top, top);
    layer_win.set_anchor(Edge::Bottom, bottom);

    match spec.exclusive_zone {
        Some(zone) => layer_win.set_exclusive_zone(zone),
        None => layer_win.auto_exclusive_zone_enable(),
    }
    layer_win.set_keyboard_mode(KeyboardMode::None);

    reparent_webview(&gtk_window, &layer_win)?;
    apply_transparency(&layer_win);
    let _ = webview.set_background_color(Some(tauri::webview::Color(0, 0, 0, 0)));

    layer_win.show_all();
    gtk_window.hide();

    LAYER_WINDOWS.with(|windows| {
        windows.borrow_mut().insert(label.to_string(), layer_win);
    });

    Ok(())
}

/// Moves the WebKit widget out of Tauri's toplevel and into the layer window.
fn reparent_webview(
    gtk_window: &gtk::ApplicationWindow,
    layer_win: &gtk::Window,
) -> Result<(), Box<dyn std::error::Error>> {
    let child = gtk_window.child().ok_or("Tauri window has no child widget")?;

    let container = child
        .dynamic_cast_ref::<gtk::Container>()
        .ok_or_else(|| format!("Tauri child {} is not a container", child.type_().name()))?;

    let widget = container
        .children()
        .first()
        .cloned()
        .ok_or("Tauri container holds no webview")?;

    container.remove(&widget);
    layer_win.add(&widget);

    Ok(())
}

fn apply_transparency(layer_win: &gtk::Window) {
    let Some(screen) = gtk::prelude::WidgetExt::screen(layer_win) else {
        return;
    };

    if let Some(rgba) = screen.rgba_visual() {
        layer_win.set_visual(Some(&rgba));
    }

    let css = gtk::CssProvider::new();
    if css
        .load_from_data(b"window { background-color: rgba(0, 0, 0, 0); }")
        .is_ok()
    {
        layer_win
            .style_context()
            .add_provider(&css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
    }
}

/// Tears down every shell surface whose label starts with one of `prefixes`,
/// closing both the layer window and the Tauri webview behind it.
pub fn destroy_layer_windows(app: &AppHandle, prefixes: &[&str]) {
    let matches = |label: &str| prefixes.iter().any(|prefix| label.starts_with(prefix));

    LAYER_WINDOWS.with(|windows| {
        let mut windows = windows.borrow_mut();
        let labels: Vec<String> = windows
            .keys()
            .filter(|label| matches(label))
            .cloned()
            .collect();

        for label in labels {
            if let Some(window) = windows.remove(&label) {
                unsafe {
                    window.destroy();
                }
            }
        }
    });

    // The hidden xdg-toplevels still exist behind each layer window.
    for (label, window) in app.webview_windows() {
        if matches(&label) {
            let _ = window.close();
        }
    }
}
