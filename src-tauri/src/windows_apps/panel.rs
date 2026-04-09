use gtk::prelude::*;
use tauri::{App, Manager, PhysicalPosition, PhysicalSize, Position, Size, WebviewWindow};
#[cfg(feature = "x11")]
use x11rb::connection::Connection;
#[cfg(feature = "x11")]
use x11rb::protocol::xproto::ConnectionExt;
#[cfg(feature = "x11")]
use x11rb::protocol::xproto::PropMode;
#[cfg(feature = "x11")]
use x11rb::wrapper::ConnectionExt as _;

use crate::monitor_manager::get_primary_monitor;

pub fn create_panel(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(app.handle()).ok_or("No primary monitor found")?;
    let primary_monitor_size = primary_monitor.size();
    let primary_monitor_position = primary_monitor.position();

    let panel_window = app
        .get_webview_window("panel")
        .expect("panel window not found");

    panel_window.set_position(Position::Physical(PhysicalPosition {
        x: primary_monitor_position.x,
        y: primary_monitor_position.y,
    }))?;

    panel_window.set_size(Size::Physical(PhysicalSize {
        width: primary_monitor_size.width,
        height: 38,
    }))?;

    panel_window.set_max_size(Some(Size::Physical(PhysicalSize {
        width: primary_monitor_size.width,
        height: 38,
    })))?;

    panel_window.set_min_size(Some(Size::Physical(PhysicalSize {
        width: primary_monitor_size.width,
        height: 38,
    })))?;

    set_window_properties(&panel_window);
    Ok(())
}

fn set_window_properties(window: &WebviewWindow) {
    let gtk_window = window.gtk_window().expect("Failed to get GTK window");

    gtk_window.set_type_hint(gdk::WindowTypeHint::Dock);
    gtk_window.set_skip_taskbar_hint(true);
    gtk_window.set_skip_pager_hint(true);
    gtk_window.set_keep_above(true);
    gtk_window.stick();
    gtk_window.set_accept_focus(false);

    // Asegurar que la ventana esté realizada antes de establecer propiedades X11
    gtk_window.realize();

    #[cfg(feature = "wayland")]
    set_wayland_properties(&gtk_window);
    #[cfg(feature = "x11")]
    set_x11_properties(&gtk_window);

    gtk_window.show();

    // Esperar un poco para dar margen a que el compositor procese la ventana del panel.
    std::thread::sleep(std::time::Duration::from_millis(250));
}

#[cfg(feature = "wayland")]
fn set_wayland_properties(_gtk_window: &gtk::ApplicationWindow) {
    // Nota: El soporte completo para layer-shell en Wayland puede requerir bindings FFI adicionales.
    // Aquí solo se indica la intención.
    //
    // Si usas gtk-layer-shell, deberías llamar a las funciones apropiadas aquí.
    // Ejemplo pseudocódigo:
    // gtk_layer_shell::init_for_window(gtk_window);
    // gtk_layer_shell::set_layer(gtk_window, gtk_layer_shell::Layer::Top);
    // gtk_layer_shell::set_anchor(gtk_window, gtk_layer_shell::Edge::Top, true);
    // gtk_layer_shell::set_exclusive_zone(gtk_window, 38);
    //
    // Si no tienes gtk-layer-shell, deberías implementarlo vía FFI o usar otra solución.
}

#[cfg(feature = "x11")]
fn set_x11_properties(gtk_window: &gtk::ApplicationWindow) {
    use gdk::prelude::*;
    const PANEL_HEIGHT: u32 = 38;

    if let Some(gdk_window) = gtk_window.window() {
        let display = gdk_window.display();

        // Usar geometría completa del monitor para calcular correctamente STRUT(_PARTIAL)
        if let Some(monitor) = display.primary_monitor() {
            let geometry = monitor.geometry();
            let monitor_x = geometry.x().max(0) as u32;
            let monitor_width = geometry.width().max(1) as u32;

            unsafe {
                use gdk::ffi;

                let window_ptr = gdk_window.as_ptr();

                // Asegurar que la ventana tenga backend nativo X11
                ffi::gdk_window_ensure_native(window_ptr);

                let top_start_x = monitor_x;
                let top_end_x = monitor_x + monitor_width - 1;

                // Escribir EWMH directamente vía X11 evita conversiones ambiguas en formato 32.
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

    // Orden EWMH: left, right, top, bottom
    let atom_wm_strut = intern("_NET_WM_STRUT")?;
    conn.change_property32(
        PropMode::REPLACE,
        xid,
        atom_wm_strut,
        atom_type_cardinal,
        &[0, 0, panel_height, 0],
    )
    .map_err(|e| e.to_string())?;

    // Orden EWMH:
    // left, right, top, bottom,
    // left_start_y, left_end_y, right_start_y, right_end_y,
    // top_start_x, top_end_x, bottom_start_x, bottom_end_x
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