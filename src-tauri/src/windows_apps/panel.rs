use gtk::prelude::*;
use tauri::{App, Manager, PhysicalPosition, PhysicalSize, Position, Size, WebviewWindow};

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
    gtk_window.show();

    // Esperar más tiempo para que todo esté completamente inicializado
    std::thread::sleep(std::time::Duration::from_millis(500));

    #[cfg(feature = "wayland")]
    set_wayland_properties(&gtk_window);
    #[cfg(feature = "x11")]
    set_x11_properties(&gtk_window);
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

    if let Some(gdk_window) = gtk_window.window() {
        let display = gdk_window.display();

        if let Some(monitor) = display.primary_monitor() {
            let geometry = monitor.geometry();
            let screen_x = geometry.x() as u32;
            let screen_width = geometry.width() as u32;

            unsafe {
                use gdk::ffi;
                use std::ffi::CString;

                let window_ptr = gdk_window.as_ptr();

                // Elimina primero cualquier STRUT previo para evitar conflictos
                if let Ok(strut_name) = CString::new("_NET_WM_STRUT") {
                    ffi::gdk_property_delete(window_ptr, ffi::gdk_atom_intern(strut_name.as_ptr(), 0));
                }
                if let Ok(strut_partial_name) = CString::new("_NET_WM_STRUT_PARTIAL") {
                    ffi::gdk_property_delete(window_ptr, ffi::gdk_atom_intern(strut_partial_name.as_ptr(), 0));
                }

                set_wm_atom_property(
                    window_ptr,
                    "_NET_WM_WINDOW_TYPE",
                    "_NET_WM_WINDOW_TYPE_DOCK",
                );

                // STRUT simple: no reservar espacio global, solo usar STRUT_PARTIAL
                let basic_strut: [u32; 4] = [0, 0, 0, 0];
                if let (Ok(strut_name), Ok(type_name)) =
                    (CString::new("_NET_WM_STRUT"), CString::new("CARDINAL"))
                {
                    ffi::gdk_property_change(
                        window_ptr,
                        ffi::gdk_atom_intern(strut_name.as_ptr(), 0),
                        ffi::gdk_atom_intern(type_name.as_ptr(), 0),
                        32,
                        ffi::GDK_PROP_MODE_REPLACE,
                        basic_strut.as_ptr() as *const u8,
                        4,
                    );
                }

                // STRUT_PARTIAL: reservar solo el área del panel en el monitor principal
                let mut strut_partial: [u32; 12] = [0; 12];
                strut_partial[2] = 38; // top
                strut_partial[8] = screen_x; // top_start_x
                strut_partial[9] = screen_x + screen_width - 1; // top_end_x

                if let (Ok(strut_partial_name), Ok(type_name)) = (
                    CString::new("_NET_WM_STRUT_PARTIAL"),
                    CString::new("CARDINAL"),
                ) {
                    ffi::gdk_property_change(
                        window_ptr,
                        ffi::gdk_atom_intern(strut_partial_name.as_ptr(), 0),
                        ffi::gdk_atom_intern(type_name.as_ptr(), 0),
                        32,
                        ffi::GDK_PROP_MODE_REPLACE,
                        strut_partial.as_ptr() as *const u8,
                        12,
                    );
                }

                let desktop_value: u32 = 0xFFFFFFFF;
                set_wm_cardinal_property(window_ptr, "_NET_WM_DESKTOP", &[desktop_value]);

                // Forzar actualización final
                ffi::gdk_display_sync(display.as_ptr());
                ffi::gdk_display_flush(display.as_ptr());
            }
        }
    }
}

#[cfg(feature = "x11")]
unsafe fn set_wm_atom_property(window_ptr: *mut gdk::ffi::GdkWindow, property: &str, value: &str) {
    use gdk::ffi;
    use std::ffi::CString;

    if let (Ok(prop_name), Ok(val_name)) = (CString::new(property), CString::new(value)) {
        let prop_atom = ffi::gdk_atom_intern(prop_name.as_ptr(), 0);
        let val_atom = ffi::gdk_atom_intern(val_name.as_ptr(), 0);
        let atom_type = ffi::gdk_atom_intern(CString::new("ATOM").unwrap().as_ptr(), 0);

        ffi::gdk_property_change(
            window_ptr,
            prop_atom,
            atom_type,
            32,
            ffi::GDK_PROP_MODE_REPLACE,
            &val_atom as *const _ as *const u8,
            1,
        );
    }
}

#[cfg(feature = "x11")]
unsafe fn set_wm_cardinal_property(
    window_ptr: *mut gdk::ffi::GdkWindow,
    property: &str,
    values: &[u32],
) {
    use gdk::ffi;
    use std::ffi::CString;

    if let (Ok(prop_name), Ok(type_name)) = (CString::new(property), CString::new("CARDINAL")) {
        let prop_atom = ffi::gdk_atom_intern(prop_name.as_ptr(), 0);
        let type_atom = ffi::gdk_atom_intern(type_name.as_ptr(), 0);

        ffi::gdk_property_change(
            window_ptr,
            prop_atom,
            type_atom,
            32,
            ffi::GDK_PROP_MODE_REPLACE,
            values.as_ptr() as *const u8,
            values.len() as i32,
        );
    }
}