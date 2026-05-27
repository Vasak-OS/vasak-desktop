use gtk::prelude::*;
use gtk_layer_shell::{Edge, Layer, LayerShell};
use tauri::{
    App, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
#[cfg(feature = "x11")]
use x11rb::connection::Connection;
#[cfg(feature = "x11")]
use x11rb::protocol::xproto::ConnectionExt;
#[cfg(feature = "x11")]
use x11rb::protocol::xproto::PropMode;
#[cfg(feature = "x11")]
use x11rb::wrapper::ConnectionExt as _;
use crate::logger::log_info;
use crate::monitor_manager::get_primary_monitor;

fn is_wayfire() -> bool {
    let desktop = std::env::var_os("XDG_CURRENT_DESKTOP")
        .or_else(|| std::env::var_os("XDG_SESSION_DESKTOP"))
        .unwrap_or_default();
    desktop.to_string_lossy().to_lowercase().contains("wayfire")
        || std::env::var_os("WAYFIRE_IPC_SOCKET").is_some()
}

#[cfg(feature = "wayland")]
fn configure_panel_via_wayfire(title: &str, x: i32, y: i32, width: u32, height: u32) {
    use crate::window_manager::wayfire_ipc::WayfireClient;

    let title = title.to_string();
    let x = x;
    let y = y;
    let width = width;
    let height = height;

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                log_info(&format!("[panel/wayfire] failed to create runtime: {e}"));
                return;
            }
        };

        rt.block_on(async move {
            log_info(&format!(
                "[panel/wayfire] connecting to Wayfire IPC for '{title}' at ({x},{y}) {width}x{height}"
            ));

            let client = match WayfireClient::connect().await {
                Ok(c) => c,
                Err(e) => {
                    log_info(&format!("[panel/wayfire] connection failed: {e}"));
                    return;
                }
            };

            let current_pid = std::process::id() as i64;
            let title_lower = title.to_lowercase();

            for attempt in 0..50 {
                let views = match client.list_views_typed().await {
                    Ok(v) => v,
                    Err(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        continue;
                    }
                };

                log_info(&format!(
                    "[panel/wayfire] attempt {attempt}: {} views, pid={current_pid} title='{title_lower}'",
                    views.len()
                ));

                let view = views.iter().find(|v| {
                    v.pid == Some(current_pid)
                        && v.title.as_deref()
                            .map(|t| t.to_lowercase() == title_lower)
                            .unwrap_or(false)
                }).cloned();

                if let Some(view) = view {
                    log_info(&format!("[panel/wayfire] found view id={}", view.id));

                    let output_id = match client.list_outputs_typed().await {
                        Ok(outputs) => outputs.iter()
                            .find(|o| o.geometry.x == x as i64 && o.geometry.y == y as i64)
                            .map(|o| o.id as u64),
                        Err(_) => None,
                    };

                    let view_id = view.id as u64;

                    match client.configure_view_coords(
                        view_id, x as i64, y as i64,
                        width as i64, height as i64,
                        output_id,
                    ).await {
                        Ok(_) => log_info("[panel/wayfire] configure_view_coords OK"),
                        Err(e) => log_info(&format!("[panel/wayfire] configure_view_coords error: {e}")),
                    }

                    match client.set_sticky(view_id, true).await {
                        Ok(_) => log_info("[panel/wayfire] set_sticky OK"),
                        Err(e) => log_info(&format!("[panel/wayfire] set_sticky error: {e}")),
                    }

                    match client.set_always_on_top(view_id, true).await {
                        Ok(_) => log_info("[panel/wayfire] set_always_on_top OK"),
                        Err(e) => log_info(&format!("[panel/wayfire] set_always_on_top error: {e}")),
                    }

                    // skip_taskbar via IPC no tiene efecto en esta versión de Wayfire
                    // (issue #1642, marcado wontfix). Solo funciona en X11/XWayland.
                    log_info("[panel/wayfire] panel configured successfully");
                    return;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            log_info("[panel/wayfire] panel view not found after 5s");
        });
    });
}

pub fn create_panel(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(app.handle()).ok_or("No primary monitor found")?;
    let primary_monitor_size = primary_monitor.size();
    let primary_monitor_position = primary_monitor.position();

    let panel_window = WebviewWindowBuilder::new(
        app,
        "panel",
        WebviewUrl::App("index.html#/panel".into()),
    )
    .title("Vasak Panel")
    .decorations(false)
    .transparent(true)
    .skip_taskbar(true)
    .always_on_top(true)
    .inner_size(primary_monitor_size.width as f64, 38.0)
    .max_inner_size(primary_monitor_size.width as f64, 38.0)
    .min_inner_size(primary_monitor_size.width as f64, 38.0)
    .position(primary_monitor_position.x as f64, primary_monitor_position.y as f64)
    .visible(false)
    .build()?;

    set_window_properties(
        &panel_window,
        "Vasak Panel".to_string(),
        primary_monitor_position.x,
        primary_monitor_position.y,
        primary_monitor_size.width,
    );

    Ok(())
}

fn set_window_properties(window: &WebviewWindow, title: String, x: i32, y: i32, width: u32) {
    let gtk_window = window.gtk_window().expect("Failed to get GTK window");

    gtk_window.set_type_hint(gdk::WindowTypeHint::Dock);
    gtk_window.set_decorated(false);
    gtk_window.set_accept_focus(false);
    gtk_window.set_skip_taskbar_hint(true);
    gtk_window.set_skip_pager_hint(true);
    gtk_window.set_app_paintable(true);

    if let Some(visual) = gtk::prelude::WidgetExt::screen(&gtk_window)
        .and_then(|s| s.rgba_visual())
    {
        gtk_window.set_visual(Some(&visual));
    }

    // X11 hints for XWayland (Wayfire) or pure X11
    #[cfg(feature = "x11")]
    {
        gtk_window.set_keep_above(true);
        gtk_window.stick();
    }

    // Wlr-layer-shell: panel como layer-surface (excluye del switcher/Alt+Tab en Wayfire)
    if gtk_layer_shell::is_supported() {
        gtk_window.init_layer_shell();
        gtk_window.set_namespace("vasak-panel");
        gtk_window.set_layer(Layer::Top);
        gtk_window.set_anchor(Edge::Top, true);
        gtk_window.set_anchor(Edge::Left, true);
        gtk_window.set_anchor(Edge::Right, true);
        gtk_window.set_exclusive_zone(38);
        gtk_window.set_keyboard_interactivity(true);
        gtk_window.set_size_request(width as i32, 38);
    }

    // Full EWMH for pure X11 (no layer-shell available)
    #[cfg(feature = "x11")]
    if !gtk_layer_shell::is_supported() {
        set_x11_properties(&gtk_window);
    }

    let _ = window.show();
    gtk_window.show_all();
    gtk_window.present();

    // On Wayfire, configure sticky, always-on-top via IPC (fallback: no-op en layer-shell)
    #[cfg(feature = "wayland")]
    if is_wayfire() && !gtk_layer_shell::is_supported() {
        configure_panel_via_wayfire(&title, x, y, width, 38);
    }
}

#[cfg(feature = "x11")]
fn set_x11_properties(gtk_window: &gtk::ApplicationWindow) {
    use gdk::prelude::*;
    const PANEL_HEIGHT: u32 = 38;

    if let Some(gdk_window) = gtk_window.window() {
        let display = gdk_window.display();

        if let Some(monitor) = display.primary_monitor() {
            let geometry = monitor.geometry();
            let monitor_x = geometry.x().max(0) as u32;
            let monitor_width = geometry.width().max(1) as u32;

            unsafe {
                use gdk::ffi;

                let window_ptr = gdk_window.as_ptr();

                ffi::gdk_window_ensure_native(window_ptr);

                let top_start_x = monitor_x;
                let top_end_x = monitor_x + monitor_width - 1;

                if let Err(err) = apply_x11_panel_ewmh(window_ptr, PANEL_HEIGHT, top_start_x, top_end_x) {
                    eprintln!("[panel/x11] Error aplicando propiedades EWMH: {}", err);
                }

                ffi::gdk_display_sync(display.as_ptr());
                ffi::gdk_display_flush(display.as_ptr());
            }
        }
    }
}

#[cfg(feature = "x11")]
fn apply_x11_panel_ewmh(
    window_ptr: *mut gdk::ffi::GdkWindow,
    panel_height: u32,
    top_start_x: u32,
    top_end_x: u32,
) -> Result<(), String> {
    let (conn, _) = x11rb::connect(None).map_err(|e| e.to_string())?;
    let xid = unsafe { gdk_x11_window_get_xid(window_ptr) };

    let intern = |name: &str| -> Result<u32, String> {
        conn.intern_atom(false, name.as_bytes())
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())
            .map(|r| r.atom)
    };

    let atom_type_atom = intern("ATOM")?;
    let atom_type_cardinal = intern("CARDINAL")?;

    let atom_wm_window_type = intern("_NET_WM_WINDOW_TYPE")?;
    let atom_wm_window_type_dock = intern("_NET_WM_WINDOW_TYPE_DOCK")?;
    conn.change_property32(
        PropMode::REPLACE,
        xid,
        atom_wm_window_type,
        atom_type_atom,
        &[atom_wm_window_type_dock],
    )
    .map_err(|e| e.to_string())?;

    let atom_wm_state = intern("_NET_WM_STATE")?;
    let states = [
        intern("_NET_WM_STATE_STICKY")?,
        intern("_NET_WM_STATE_SKIP_TASKBAR")?,
        intern("_NET_WM_STATE_SKIP_PAGER")?,
        intern("_NET_WM_STATE_ABOVE")?,
    ];
    conn.change_property32(
        PropMode::REPLACE,
        xid,
        atom_wm_state,
        atom_type_atom,
        &states,
    )
    .map_err(|e| e.to_string())?;

    let atom_wm_desktop = intern("_NET_WM_DESKTOP")?;
    conn.change_property32(
        PropMode::REPLACE,
        xid,
        atom_wm_desktop,
        atom_type_cardinal,
        &[0xFFFFFFFF],
    )
    .map_err(|e| e.to_string())?;

    let atom_wm_strut = intern("_NET_WM_STRUT")?;
    conn.change_property32(
        PropMode::REPLACE,
        xid,
        atom_wm_strut,
        atom_type_cardinal,
        &[0, 0, panel_height, 0],
    )
    .map_err(|e| e.to_string())?;

    let atom_wm_strut_partial = intern("_NET_WM_STRUT_PARTIAL")?;
    let strut_partial = [
        0,
        0,
        panel_height,
        0,
        0,
        0,
        0,
        0,
        top_start_x,
        top_end_x,
        0,
        0,
    ];
    conn.change_property32(
        PropMode::REPLACE,
        xid,
        atom_wm_strut_partial,
        atom_type_cardinal,
        &strut_partial,
    )
    .map_err(|e| e.to_string())?;

    let atom_win_layer = intern("_WIN_LAYER")?;
    conn.change_property32(
        PropMode::REPLACE,
        xid,
        atom_win_layer,
        atom_type_cardinal,
        &[6],
    )
    .map_err(|e| e.to_string())?;

    conn.flush().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(feature = "x11")]
unsafe fn gdk_x11_window_get_xid(window_ptr: *mut gdk::ffi::GdkWindow) -> u32 {
    unsafe extern "C" {
        fn gdk_x11_window_get_xid(window: *mut gdk::ffi::GdkWindow) -> libc::c_ulong;
    }

    unsafe { gdk_x11_window_get_xid(window_ptr) as u32 }
}
