use crate::window_manager;
use crate::window_manager::delta::WindowDelta;
use crate::window_manager::{WindowInfo, WindowManagerBackend};
use crate::structs::CachedWindowList;
use crate::logger::{log_info, log_error};
use window_manager::WindowManager;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tauri::Emitter;

/// Debounce window for coalescing rapid sequential changes (50ms).
const DEBOUNCE_MS: u64 = 50;

pub fn setup_windows_monitoring(
    window_manager: Arc<RwLock<WindowManager>>,
    app_handle: tauri::AppHandle,
    cached_windows: Arc<parking_lot::RwLock<Option<CachedWindowList>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    log_info("Configurando monitoreo de ventanas");
    let (tx, rx) = channel();

    {
        let mut wm = window_manager.write().unwrap_or_else(|error| error.into_inner());
        wm.backend.setup_event_monitoring(tx)?;
        log_info("Monitoreo de eventos de ventanas establecido");
    }

    // Event-driven thread: on Wayfire events, trigger a window list refresh with debounce.
    let event_handle = app_handle.clone();
    let event_cached = Arc::clone(&cached_windows);

    std::thread::spawn(move || {
        let mut last_emit_time = Instant::now() - Duration::from_millis(DEBOUNCE_MS + 1);
        let mut last_snapshot: Vec<WindowInfo> = Vec::new();

        // Initialize snapshot from cache if available
        if let Some(cached) = event_cached.read().as_ref() {
            last_snapshot = cached.windows.clone();
        }

        // Use recv() in a loop so we can also call try_recv() for draining
        while rx.recv().is_ok() {
            let now = Instant::now();
            let elapsed = now.duration_since(last_emit_time);

            // Debounce: if last emission was <50ms ago, wait out the remaining time
            // and drain any additional events that arrive during the wait (coalescing)
            if elapsed < Duration::from_millis(DEBOUNCE_MS) {
                let remaining = Duration::from_millis(DEBOUNCE_MS) - elapsed;
                std::thread::sleep(remaining);
            }
            // Drain any additional events that arrived (coalesce rapid sequential changes)
            while rx.try_recv().is_ok() {}

            // Fetch windows without holding the main RwLock (Requirement 13.3).
            // WaylandManager is stateless, so we create a fresh instance for the IPC call.
            let windows = match window_manager::wayland::WaylandManager::default().get_window_list() {
                Ok(w) => w,
                Err(e) => {
                    log_error(&format!("Error obteniendo ventanas (evento): {}", e));
                    continue;
                }
            };

            // Update the shared cached window list (brief lock <1ms, Requirement 13.2)
            {
                let mut cache = event_cached.write();
                *cache = Some(CachedWindowList {
                    windows: windows.clone(),
                    updated_at: Instant::now(),
                });
            }

            // Compute delta and emit (Requirement 4.1)
            if let Some(delta) = WindowDelta::compute(&last_snapshot, &windows) {
                let _ = event_handle.emit("window-delta", &delta);
                last_emit_time = Instant::now();
            }
            last_snapshot = windows;
        }
    });

    // Polling thread: periodic 1000ms poll with delta computation and 50ms debounce.
    // Does NOT hold the main RwLock during the Wayfire IPC call (Requirement 13.3).
    let polling_cached = Arc::clone(&cached_windows);
    let polling_handle = app_handle.clone();

    std::thread::spawn(move || {
        let mut last_snapshot: Vec<WindowInfo> = Vec::new();
        let mut last_emit_time = Instant::now() - Duration::from_millis(DEBOUNCE_MS + 1);

        loop {
            // Fetch windows OUTSIDE the lock (slow I/O via Wayfire IPC).
            // WaylandManager is stateless, so a default instance works fine.
            let windows = match window_manager::wayland::WaylandManager::default().get_window_list()
            {
                Ok(w) => w,
                Err(error) => {
                    log_error(&format!("Error obteniendo snapshot de ventanas: {}", error));
                    std::thread::sleep(Duration::from_millis(1000));
                    continue;
                }
            };

            // Update the shared cached window list (brief write lock <1ms, Requirement 13.2)
            {
                let mut cache = polling_cached.write();
                *cache = Some(CachedWindowList {
                    windows: windows.clone(),
                    updated_at: Instant::now(),
                });
            }

            // Compute delta between previous snapshot and current
            if let Some(delta) = WindowDelta::compute(&last_snapshot, &windows) {
                // 50ms debounce/coalescing (Requirement 4.5)
                let now = Instant::now();
                let elapsed = now.duration_since(last_emit_time);
                if elapsed < Duration::from_millis(DEBOUNCE_MS) {
                    let remaining = Duration::from_millis(DEBOUNCE_MS) - elapsed;
                    std::thread::sleep(remaining);
                }

                let _ = polling_handle.emit("window-delta", &delta);
                last_emit_time = Instant::now();
            }
            last_snapshot = windows;

            std::thread::sleep(Duration::from_millis(1000));
        }
    });

    Ok(())
}

// Battery, Music, Notifications moved to AppletManager/Applets

pub fn setup_dbus_service(app_handle: tauri::AppHandle) {
    log_info("Iniciando servicio D-Bus");
    tauri::async_runtime::spawn(async move {
        if let Err(e) = crate::dbus_service::start_dbus_service(app_handle).await {
            log_error(&format!("Error al iniciar servicio D-Bus: {}", e));
        }
    });
}
