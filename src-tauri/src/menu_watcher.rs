use inotify::{Inotify, WatchMask};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::inotify_rafaga::esperar_rafaga;
use crate::logger::{log_error, log_info};
use crate::menu_manager::{applications_dirs, invalidate_menu_cache};

/// Emitted when the installed application list changes.
pub const MENU_CHANGED_EVENT: &str = "menu-items-changed";

/// Package managers write many files in a row, so wait for the burst to end
/// before rescanning rather than rescanning per file.
const SETTLE: Duration = Duration::from_millis(750);

/// Watches the XDG applications directories and refreshes the menu when an app
/// is installed, removed or updated.
///
/// This is what makes it safe for the menu window to be hidden instead of
/// destroyed. Previously every open rebuilt the window from scratch, which
/// re-ran the full .desktop scan — slow, but it did mean a newly installed app
/// showed up. Keeping the window alive removes that accidental refresh, so the
/// list has to be invalidated deliberately. The result is better on both
/// counts: opening the menu no longer scans anything, and a new app appears
/// immediately instead of only after the menu is closed and reopened.
pub fn watch_application_dirs(app: &AppHandle) {
    let app = app.clone();

    std::thread::spawn(move || {
        let mut inotify = match Inotify::init() {
            Ok(inotify) => inotify,
            Err(error) => {
                log_error(&format!(
                    "No se pudo iniciar inotify para el menú: {}. La lista de aplicaciones no se actualizará sola.",
                    error
                ));
                return;
            }
        };

        let mask = WatchMask::CREATE
            | WatchMask::DELETE
            | WatchMask::MOVED_TO
            | WatchMask::MOVED_FROM
            | WatchMask::CLOSE_WRITE;

        let mut watched = 0;
        for dir in applications_dirs() {
            match inotify.watches().add(&dir, mask) {
                Ok(_) => watched += 1,
                // Several XDG entries routinely don't exist; that isn't an error.
                Err(error) => log_info(&format!(
                    "Sin vigilancia sobre {}: {}",
                    dir.display(),
                    error
                )),
            }
        }

        if watched == 0 {
            log_error("Ningún directorio de aplicaciones pudo vigilarse");
            return;
        }

        log_info(&format!(
            "Vigilando {} directorio(s) de aplicaciones para actualizar el menú",
            watched
        ));

        let mut buffer = [0u8; 4096];

        loop {
            // Bloquea hasta que algo cambie. Antes este bucle sondeaba a 5 Hz
            // para siempre —unos 144 mil despertares en ocho horas— y casi
            // todos eran para descubrir que no había nada.
            match esperar_rafaga(&mut inotify, &mut buffer, SETTLE) {
                Ok(()) => {
                    invalidate_menu_cache();
                    log_info("Cambió la lista de aplicaciones; menú invalidado");
                    let _ = app.emit(MENU_CHANGED_EVENT, ());
                }
                Err(error) => {
                    log_error(&format!("Vigilancia del menú interrumpida: {}", error));
                    return;
                }
            }
        }
    });
}
