use glib::ffi;
use std::boxed::Box;

/// Schedules a closure to run on the GTK main thread via `g_main_context_invoke_full`.
///
/// # Safety
///
/// The closure may capture non-Send types (e.g. `gtk::Window`), but it is only
/// ever invoked on the main context's owning thread (the GTK main thread).
pub(crate) unsafe fn invoke_on_main<F>(f: F)
where
    F: FnOnce() + 'static,
{
    unsafe extern "C" fn trampoline<F: FnOnce() + 'static>(data: ffi::gpointer) -> ffi::gboolean {
        let func = &mut *(data as *mut Option<F>);
        if let Some(f) = func.take() {
            f();
        }
        ffi::G_SOURCE_REMOVE
    }
    unsafe extern "C" fn destroy<F: FnOnce() + 'static>(data: ffi::gpointer) {
        let _ = Box::<Option<F>>::from_raw(data as *mut _);
    }

    let func = Box::into_raw(Box::new(Some(f)));
    ffi::g_main_context_invoke_full(
        std::ptr::null_mut(),
        ffi::G_PRIORITY_DEFAULT,
        Some(trampoline::<F>),
        func as ffi::gpointer,
        Some(destroy::<F>),
    );
}
