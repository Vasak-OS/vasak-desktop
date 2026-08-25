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
    /// Gap from each anchored edge, in the same order as `anchors`.
    pub margins: (i32, i32, i32, i32),
    /// Whether the surface takes keyboard focus. A popup needs it so Escape
    /// reaches it and so losing focus is something it can notice; a panel or a
    /// wallpaper must never steal it.
    pub keyboard: KeyboardMode,
    /// Popups are built once and shown on demand, so they start hidden: the
    /// alternative is the control centre flashing onto the screen at login.
    pub start_hidden: bool,
    /// Hide as soon as the surface loses focus or Escape is pressed. This is
    /// what makes a popup behave like a popup — it used to stay open until it
    /// was toggled again, over whatever the person clicked next.
    pub dismiss_on_unfocus: bool,
}

impl Default for LayerSpec {
    fn default() -> Self {
        Self {
            namespace: "vasak",
            layer: Layer::Top,
            anchors: (false, false, false, false),
            exclusive_zone: None,
            margins: (0, 0, 0, 0),
            keyboard: KeyboardMode::None,
            start_hidden: false,
            dismiss_on_unfocus: false,
        }
    }
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

    let (left, right, top, bottom) = spec.anchors;

    // Layer-shell asks the compositor for the surface size on every axis that is
    // not anchored to both of its edges, and it takes that size from this
    // window. `inner_size` above applies to the throwaway toplevel, not to this
    // one, so without a request here the axis collapses to what WebKit asks for,
    // which is nothing: that is how the control centre ended up as a sliver in
    // the corner. -1 is GTK for "no request", and leaves the stretched axes to
    // the compositor.
    layer_win.set_size_request(
        if left && right { -1 } else { width as i32 },
        if top && bottom { -1 } else { height as i32 },
    );

    layer_win.init_layer_shell();
    layer_win.set_monitor(gdk_monitor);
    layer_win.set_namespace(spec.namespace);
    layer_win.set_layer(spec.layer);

    layer_win.set_anchor(Edge::Left, left);
    layer_win.set_anchor(Edge::Right, right);
    layer_win.set_anchor(Edge::Top, top);
    layer_win.set_anchor(Edge::Bottom, bottom);

    let (margin_left, margin_right, margin_top, margin_bottom) = spec.margins;
    layer_win.set_layer_shell_margin(Edge::Left, margin_left);
    layer_win.set_layer_shell_margin(Edge::Right, margin_right);
    layer_win.set_layer_shell_margin(Edge::Top, margin_top);
    layer_win.set_layer_shell_margin(Edge::Bottom, margin_bottom);

    match spec.exclusive_zone {
        Some(zone) => layer_win.set_exclusive_zone(zone),
        None => layer_win.auto_exclusive_zone_enable(),
    }
    layer_win.set_keyboard_mode(spec.keyboard);

    reparent_webview(&gtk_window, &layer_win)?;
    apply_transparency(&layer_win);
    let _ = webview.set_background_color(Some(tauri::webview::Color(0, 0, 0, 0)));

    if spec.dismiss_on_unfocus {
        // Escape and focus loss are handled here rather than in the page: the
        // surface owns the keyboard, and a click that lands on another window
        // never reaches the webview at all.
        layer_win.connect_key_press_event(|window, event| {
            if event.keyval() == gdk::keys::constants::Escape {
                window.hide();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });

        layer_win.connect_focus_out_event(|window, _| {
            window.hide();
            glib::Propagation::Proceed
        });
    }

    layer_win.show_all();
    if spec.start_hidden {
        layer_win.hide();
    }
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

    // Detrás de cada superficie de capa queda su ventana de Tauri.
    //
    // `close()` **pide** el cierre: emite el evento y la baja la procesa el
    // bucle más tarde, así que la etiqueta sigue ocupada al volver de acá y
    // recrearla falla. `destroy()` la cierra sin preguntar, que es lo que
    // corresponde cuando la estamos rehaciendo nosotros.
    for (label, window) in app.webview_windows() {
        if matches(&label) {
            if let Err(error) = window.destroy() {
                log::error!("No se pudo cerrar {label}: {error}");
            }
        }
    }
}

/// Shows a layer-shell surface that was built hidden, and takes focus so it can
/// be dismissed again.
pub fn show_layer_window(label: &str) {
    let label = label.to_string();
    unsafe {
        crate::gtk_utils::invoke_on_main(move || {
            LAYER_WINDOWS.with(|windows| {
                if let Some(window) = windows.borrow().get(&label) {
                    window.show_all();
                    window.present();
                }
            });
        });
    }
}

pub fn hide_layer_window(label: &str) {
    let label = label.to_string();
    unsafe {
        crate::gtk_utils::invoke_on_main(move || {
            LAYER_WINDOWS.with(|windows| {
                if let Some(window) = windows.borrow().get(&label) {
                    window.hide();
                }
            });
        });
    }
}

/// Whether the surface is on screen.
///
/// Only meaningful on the GTK main thread, which is where the registry lives;
/// callers on another thread get `None` and should toggle blind rather than
/// guess.
pub fn layer_window_visible(label: &str) -> Option<bool> {
    LAYER_WINDOWS.try_with(|windows| {
        windows
            .borrow()
            .get(label)
            .map(gtk::prelude::WidgetExt::is_visible)
    })
    .ok()
    .flatten()
}

/// Whether the surface was built.
///
/// Main-thread only, like [`layer_window_visible`]: the registry is thread-local
/// and every other thread sees an empty one, so this answers `false` there
/// whatever is actually on screen. Callers off the main thread must marshal onto
/// it — `AppHandle::run_on_main_thread` — rather than trust the answer.
pub fn layer_window_exists(label: &str) -> bool {
    LAYER_WINDOWS
        .try_with(|windows| windows.borrow().contains_key(label))
        .unwrap_or(false)
}
