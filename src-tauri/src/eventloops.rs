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

/// Shared state for unified delta computation — single source of truth
/// for both the event-driven and polling threads.
struct EmittedState {
    snapshot: Vec<WindowInfo>,
    last_emit: Instant,
}

/// Compute delta against the shared snapshot and emit if changed.
/// Both threads (event and polling) call this — the shared snapshot
/// guarantees each window change produces exactly one `window-delta`.
fn emit_if_changed(
    emitted: &RwLock<EmittedState>,
    handle: &tauri::AppHandle,
    windows: &[WindowInfo],
) {
    // Claim the change under the lock, then release it before sleeping or
    // emitting. The debounce used to be a sleep held *inside* the write lock,
    // so the event thread and the polling thread blocked each other for up to
    // 50 ms on every window change.
    //
    // Updating the snapshot before releasing still guarantees exactly one
    // emission per change: whoever claims it first leaves the other computing
    // against the new snapshot, which yields no delta.
    let claimed = {
        let mut state = emitted.write().unwrap_or_else(|e| e.into_inner());

        let Some(delta) = WindowDelta::compute(&state.snapshot, windows) else {
            return;
        };

        let debounce = Duration::from_millis(DEBOUNCE_MS);
        let wait = debounce
            .checked_sub(Instant::now().duration_since(state.last_emit))
            .unwrap_or_default();

        // Record when the emission will actually happen, not now.
        state.last_emit = Instant::now() + wait;
        state.snapshot = windows.to_vec();

        (delta, wait)
    };

    let (delta, wait) = claimed;

    if !wait.is_zero() {
        std::thread::sleep(wait);
    }

    let _ = handle.emit("window-delta", &delta);
}

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

    // Shared emission state — single delta stream for both threads.
    let emitted = Arc::new(RwLock::new(EmittedState {
        snapshot: Vec::new(),
        last_emit: Instant::now() - Duration::from_millis(DEBOUNCE_MS + 1),
    }));

    // Seed snapshot from cache if available (initial state).
    if let Some(cached) = cached_windows.read().as_ref() {
        if let Ok(mut state) = emitted.write() {
            state.snapshot = cached.windows.clone();
        }
    }

    // -- Event-driven thread ------------------------------------------------
    // On Wayfire events, trigger a window list refresh with debounce/coalescing.
    let event_emitted = Arc::clone(&emitted);
    let event_handle = app_handle.clone();
    let event_cached = Arc::clone(&cached_windows);

    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            // Drain any additional events that arrived (coalesce rapid sequences)
            while rx.try_recv().is_ok() {}

            // Fetch windows without holding the main RwLock (Req 13.3).
            let windows = match window_manager::wayland::WaylandManager::default().get_window_list()
            {
                Ok(w) => w,
                Err(e) => {
                    log_error(&format!("Error obteniendo ventanas (evento): {}", e));
                    continue;
                }
            };

            // Update shared cached window list (brief lock <1ms, Req 13.2)
            {
                let mut cache = event_cached.write();
                *cache = Some(CachedWindowList {
                    windows: windows.clone(),
                    updated_at: Instant::now(),
                });
            }

            // Compute delta against shared snapshot and emit (Req 4.1)
            emit_if_changed(&event_emitted, &event_handle, &windows);
        }
    });

    // -- Polling thread ------------------------------------------------------
    // Periodic 1000ms reconciliation — does NOT maintain its own delta stream.
    // Reuses the shared EmittedState, so it never duplicates an already-emitted change.
    let polling_emitted = Arc::clone(&emitted);
    let polling_handle = app_handle.clone();
    let polling_cached = Arc::clone(&cached_windows);

    std::thread::spawn(move || {
        loop {
            // Fetch windows outside the lock (slow I/O via Wayfire IPC).
            let windows = match window_manager::wayland::WaylandManager::default().get_window_list()
            {
                Ok(w) => w,
                Err(error) => {
                    log_error(&format!("Error obteniendo snapshot de ventanas: {}", error));
                    std::thread::sleep(Duration::from_millis(1000));
                    continue;
                }
            };

            // Update shared cached window list (brief write lock <1ms, Req 13.2)
            {
                let mut cache = polling_cached.write();
                *cache = Some(CachedWindowList {
                    windows: windows.clone(),
                    updated_at: Instant::now(),
                });
            }

            // Reconcile — may emit if event thread fell behind; otherwise no-op.
            emit_if_changed(&polling_emitted, &polling_handle, &windows);

            std::thread::sleep(Duration::from_millis(1000));
        }
    });

    Ok(())
}

pub fn setup_dbus_service(app_handle: tauri::AppHandle) {
    log_info("Iniciando servicio D-Bus");
    tauri::async_runtime::spawn(async move {
        if let Err(e) = crate::dbus_service::start_dbus_service(app_handle).await {
            log_error(&format!("Error al iniciar servicio D-Bus: {}", e));
        }
    });
}
