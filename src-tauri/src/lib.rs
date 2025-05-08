use tauri::Manager;
use gtk::traits::GtkWindowExt;

fn set_window_properties(window: &tauri::WebviewWindow) {
    let gtk_window = window.gtk_window().expect("Failed to get GTK window");

    gtk_window.set_type_hint(gdk::WindowTypeHint::Desktop);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .invoke_handler(tauri::generate_handler![])
        .setup(move |app| {
            let window = app
                .get_webview_window("main")
                .expect("main window not found");
            set_window_properties(&window);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
