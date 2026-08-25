use gdk::prelude::*;
use std::cell::Cell;
use std::time::Duration;
use tauri::{AppHandle, Manager, Monitor};

use crate::logger::{log_debug, log_error, log_info};
use crate::windows_apps::desktop::create_desktops;
use crate::windows_apps::panel::create_panels;
use crate::windows_apps::shell_layer::destroy_layer_windows;

/// Outputs settle in bursts — plugging a screen in can emit several signals, and
/// a mode change arrives as a remove followed by an add. Rebuilding on each one
/// would tear the shell down and back up repeatedly, so coalesce them.
const REBUILD_DEBOUNCE: Duration = Duration::from_millis(400);

pub fn get_monitors(app: &AppHandle) -> Option<Vec<Monitor>> {
    log_debug("Detectando monitores disponibles");
    let monitors = app.available_monitors().ok()?;
    log_info(&format!("Detectados {} monitores", monitors.len()));
    for (i, monitor) in monitors.iter().enumerate() {
        log_debug(&format!(
            "  Monitor {}: {}x{} en ({},{})",
            i,
            monitor.size().width,
            monitor.size().height,
            monitor.position().x,
            monitor.position().y
        ));
    }
    Some(monitors)
}

/// The monitor the shell puts the panel on.
///
/// This used to guess "whichever monitor sits at (0,0)", which is only right by
/// coincidence: with several screens the one at the origin is simply the
/// leftmost/topmost in the layout, not the one the user set as primary. The
/// panel and the control centre then appear on the wrong screen. Ask the
/// platform first and keep the old guess only as a fallback.
pub fn get_primary_monitor(app: &AppHandle) -> Option<Monitor> {
    log_debug("Obteniendo monitor primario");

    if let Ok(Some(primary)) = app.primary_monitor() {
        log_debug(&format!(
            "Monitor primario (reportado por el sistema): {}x{} en ({},{})",
            primary.size().width,
            primary.size().height,
            primary.position().x,
            primary.position().y
        ));
        return Some(primary);
    }

    log_debug("El sistema no reporta monitor primario; se usa el de la posición (0,0)");

    if let Some(monitors) = get_monitors(app) {
        let primary = monitors
            .iter()
            .find(|monitor| monitor.position().x == 0 && monitor.position().y == 0)
            .or_else(|| monitors.first())
            .cloned();
        if let Some(ref mon) = primary {
            log_debug(&format!(
                "Monitor primario: {}x{}",
                mon.size().width,
                mon.size().height
            ));
        }
        primary
    } else {
        log_error("No se pudieron obtener monitores");
        None
    }
}

/// Stable window label for a surface on `monitor`.
///
/// The primary keeps the bare name so existing routes and callers that address
/// "panel" or "desktop" keep working; the rest are suffixed by index.
pub fn label_for(kind: &str, monitor: &Monitor, primary: &Monitor, index: usize) -> String {
    if monitor.position() == primary.position() {
        kind.to_string()
    } else {
        format!("{}_{}", kind, index)
    }
}

/// Finds the GDK monitor matching a Tauri monitor.
///
/// Tauri reports physical pixels while GDK geometry is logical, so the position
/// has to be scaled down before comparing — otherwise nothing matches on a
/// HiDPI screen and the surface never gets created.
pub fn find_gdk_monitor(monitor: &Monitor) -> Option<gdk::Monitor> {
    let position = monitor.position();
    let scale = monitor.scale_factor();
    let logical_x = (position.x as f64 / scale) as i32;
    let logical_y = (position.y as f64 / scale) as i32;

    let display = gdk::Display::default()?;
    for index in 0..display.n_monitors() {
        let candidate = display.monitor(index)?;
        let geometry = candidate.geometry();
        if geometry.x() == logical_x && geometry.y() == logical_y {
            return Some(candidate);
        }
    }
    None
}

/// Rebuilds every panel and desktop for the monitors currently connected.
/// Los prefijos de las superficies que se rehacen al cambiar los monitores.
const SUPERFICIES: [&str; 2] = ["panel", "desktop"];

/// Cada cuánto se vuelve a mirar si las etiquetas quedaron libres.
const ESPERA_ENTRE_INTENTOS: Duration = Duration::from_millis(120);

/// Cuántas veces: medio segundo en total, de sobra para lo que tarda el cierre
/// de una ventana y poco como para no dejar la pantalla vacía si algo falló.
const INTENTOS: u32 = 4;

pub fn rebuild_shell_surfaces(app: &AppHandle) {
    log_info("Reconstruyendo paneles y escritorios por cambio de monitores");

    destroy_layer_windows(app, &SUPERFICIES);
    recrear_cuando_se_liberen(app.clone(), 0);
}

/// Qué etiquetas siguen ocupadas.
///
/// Cerrar una ventana no libera su etiqueta en el mismo instante: la baja la
/// procesa el bucle de eventos. Recrear antes de eso falla con «a webview with
/// label `panel` already exists», y ahí no queda ni panel ni escritorio: la
/// pantalla se ve negra hasta reiniciar la sesión. Es exactamente lo que pasaba
/// al desconectar un monitor.
fn ocupadas<'a>(etiquetas: impl Iterator<Item = &'a str>, prefijos: &[&str]) -> Vec<String> {
    etiquetas
        .filter(|etiqueta| prefijos.iter().any(|prefijo| etiqueta.starts_with(prefijo)))
        .map(str::to_string)
        .collect()
}

fn recrear_cuando_se_liberen(app: AppHandle, intento: u32) {
    let ventanas = app.webview_windows();
    let pendientes = ocupadas(ventanas.keys().map(String::as_str), &SUPERFICIES);

    if !pendientes.is_empty() {
        if intento < INTENTOS {
            gtk::glib::timeout_add_local_once(ESPERA_ENTRE_INTENTOS, move || {
                recrear_cuando_se_liberen(app, intento + 1);
            });
            return;
        }

        // Se intenta igual: una etiqueta trabada deja esa superficie afuera,
        // pero las demás pueden crearse y algo es mejor que una pantalla negra.
        log_error(&format!(
            "Estas ventanas no terminaron de cerrarse: {}. Se recrea igual.",
            pendientes.join(", ")
        ));
    }

    if let Err(error) = create_desktops(&app) {
        log_error(&format!("No se pudieron recrear los escritorios: {}", error));
    }
    if let Err(error) = create_panels(&app) {
        log_error(&format!("No se pudieron recrear los paneles: {}", error));
    }
}

/// Watches for monitors being connected, disconnected or reconfigured and
/// rebuilds the shell surfaces to match.
///
/// Without this the panels and wallpapers stay bound to whatever outputs existed
/// at login: a screen plugged in afterwards gets nothing, and one unplugged
/// leaves its surfaces behind.
pub fn watch_monitor_changes(app: &AppHandle) {
    let Some(display) = gdk::Display::default() else {
        log_error("Sin display GDK: no se pueden vigilar los cambios de monitores");
        return;
    };

    let schedule_rebuild = {
        let app = app.clone();
        // A pending flag rather than a timer per event, so a burst of signals
        // results in exactly one rebuild.
        let pending = std::rc::Rc::new(Cell::new(false));

        move || {
            if pending.replace(true) {
                return;
            }

            let app = app.clone();
            let pending = pending.clone();
            gtk::glib::timeout_add_local_once(REBUILD_DEBOUNCE, move || {
                pending.set(false);
                rebuild_shell_surfaces(&app);
            });
        }
    };

    let on_added = schedule_rebuild.clone();
    display.connect_monitor_added(move |_, _| on_added());

    let on_removed = schedule_rebuild.clone();
    display.connect_monitor_removed(move |_, _| on_removed());

    // Resolution, scale or position changes don't add or remove a monitor, so
    // they need watching per-output as well.
    for index in 0..display.n_monitors() {
        if let Some(monitor) = display.monitor(index) {
            let on_geometry = schedule_rebuild.clone();
            monitor.connect_geometry_notify(move |_| on_geometry());
        }
    }

    log_info("Vigilancia de cambios de monitores activa");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cerrar una ventana no libera su etiqueta en el acto, y recrearla antes
    /// falla con «already exists»: sin panel ni escritorio, la pantalla queda
    /// negra. Esto es lo que decide si conviene esperar un poco más.
    #[test]
    fn una_etiqueta_del_shell_todavia_ocupada_se_reconoce() {
        let etiquetas = ["panel", "menu", "control_center"];

        assert_eq!(
            ocupadas(etiquetas.into_iter(), &SUPERFICIES),
            vec!["panel".to_string()]
        );
    }

    #[test]
    fn los_escritorios_de_los_otros_monitores_cuentan() {
        // En un monitor secundario la etiqueta lleva el índice, y esa ventana
        // también hay que esperar a que se cierre.
        let etiquetas = ["desktop", "desktop_1", "desktop_2"];

        assert_eq!(ocupadas(etiquetas.into_iter(), &SUPERFICIES).len(), 3);
    }

    #[test]
    fn las_ventanas_que_no_son_del_shell_no_frenan_nada() {
        // El menú, el centro de control o un applet abierto no tienen nada que
        // ver con rehacer las superficies: esperarlos sería esperar para
        // siempre.
        let etiquetas = ["menu", "applet_network", "systray_popup", "vsk_context_menu"];

        assert!(ocupadas(etiquetas.into_iter(), &SUPERFICIES).is_empty());
    }

    #[test]
    fn sin_ventanas_no_hay_nada_que_esperar() {
        assert!(ocupadas(std::iter::empty(), &SUPERFICIES).is_empty());
    }
}
