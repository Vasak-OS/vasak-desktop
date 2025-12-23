use super::{WindowInfo, WindowManagerBackend};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use wayland_client::protocol::{wl_output, wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};

// Import wlr protocols
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

#[derive(Debug, Clone)]
struct ToplevelInfo {
    handle: ZwlrForeignToplevelHandleV1,
    title: String,
    app_id: String,
    is_maximized: bool,
    is_minimized: bool,
    is_activated: bool,
    is_fullscreen: bool,
}

impl ToplevelInfo {
    fn new(handle: ZwlrForeignToplevelHandleV1) -> Self {
        Self {
            handle,
            title: String::new(),
            app_id: String::new(),
            is_maximized: false,
            is_minimized: false,
            is_activated: false,
            is_fullscreen: false,
        }
    }

    fn to_window_info(&self, id: &str) -> WindowInfo {
        WindowInfo {
            id: id.to_string(),
            title: self.title.clone(),
            is_minimized: self.is_minimized,
            icon: self.app_id.clone(),
            demands_attention: None, // Wayland doesn't have direct equivalent in this protocol
        }
    }

    fn should_show(&self) -> bool {
        let skip_apps = [
            "vpanel",
            "tauri", 
            "vasak-control-center",
            "plank",
            "docky",
            "cairo-dock",
            "polybar",
            "waybar",
            "tint2",
            "plasmashell",
            "krunner",
            "systemsettings",
            "kwin",
            "plasma-desktop"
        ];
        
        // Also skip empty app_ids if title is also empty (ghost windows)
        if self.app_id.is_empty() && self.title.is_empty() {
             return false;
        }

        let app_id_lower = self.app_id.to_lowercase();
        !skip_apps.iter().any(|app| app_id_lower.contains(app))
    }
}

struct AppState {
    toplevels: HashMap<u32, ToplevelInfo>,
    manager: Option<ZwlrForeignToplevelManagerV1>,
    seat: Option<wl_seat::WlSeat>,
    event_sender: Option<Sender<()>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            toplevels: HashMap::new(),
            manager: None,
            seat: None,
            event_sender: None,
        }
    }
}

pub struct WaylandManager {
    conn: Connection,
    event_queue: EventQueue<AppState>,
    state: Arc<Mutex<AppState>>,
}

impl WaylandManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::connect_to_env()
            .map_err(|e| format!("Failed to connect to Wayland: {}", e))?;
        
        let event_queue = conn.new_event_queue::<AppState>();
        let qh = event_queue.handle();
        
        let _registry = conn.display().get_registry(&qh, ());
        
        let state = Arc::new(Mutex::new(AppState::new()));

        Ok(WaylandManager {
            conn,
            event_queue,
            state,
        })
    }

    pub fn setup_protocol_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Dispatch pending events to set up protocol bindings
        log::info!("Dispatching events to discover available protocols...");
        self.event_queue.blocking_dispatch(&mut *self.state.lock().unwrap())
            .map_err(|e| format!("Failed to dispatch events: {}", e))?;
        
        // Check if manager is available
        let state_guard = self.state.lock().unwrap();
        if state_guard.manager.is_some() {
            log::info!("Using wlr-foreign-toplevel-management protocol");
            return Ok(());
        }
        
        drop(state_guard);
        Err("wlr-foreign-toplevel-management protocol not available.".into())
    }
}

impl WindowManagerBackend for WaylandManager {
    fn get_window_list(&mut self) -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>> {

        if let Err(e) = self.event_queue.blocking_dispatch(&mut *self.state.lock().unwrap()) {
             log::warn!("Failed to dispatch events during get_window_list: {}", e);
        }

        let state = self.state.lock().unwrap();
        let mut windows: Vec<WindowInfo> = Vec::new();
        
        for (id, toplevel) in &state.toplevels {
            if toplevel.should_show() {
                windows.push(toplevel.to_window_info(&id.to_string()));
            }
        }
        
        windows.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(windows)
    }

    fn setup_event_monitoring(&mut self, tx: Sender<()>) -> Result<(), Box<dyn std::error::Error>> {
        match self.setup_protocol_bindings() {
            Ok(_) => {
                log::info!("Wayland window management initialized successfully");
            }
            Err(e) => {
                log::error!("Failed to initialize Wayland protocols: {}", e);
                return Err(e);
            }
        }
        
        {
            let mut state = self.state.lock().unwrap();
            state.event_sender = Some(tx);
        }

        let state_clone = Arc::clone(&self.state);
        
        let conn_clone = self.conn.clone();
        
        std::thread::spawn(move || {
            let event_queue = conn_clone.new_event_queue::<AppState>();
            let qh = event_queue.handle();
            let _registry = conn_clone.display().get_registry(&qh, ());
            let mut event_queue = event_queue;

            loop {
                let guard = state_clone.lock().unwrap();
                drop(guard); // Ensure lock is dropped
                
                match event_queue.dispatch_pending(&mut *state_clone.lock().unwrap()) {
                    Ok(_) => {},
                    Err(e) => {
                        log::error!("Error dispatching pending: {}", e);
                        break;
                    }
                }

                let _ = conn_clone.flush();

                match event_queue.prepare_read() {
                    Some(guard) => {
                        if let Err(e) = guard.read() {
                                 log::error!("Error reading events: {}", e);
                                 break;
                        }
                    },
                    None => {
                        continue;
                    }
                }
            }
        });

        Ok(())
    }

    fn toggle_window(&self, win_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let id: u32 = win_id.parse()
            .map_err(|_| "Invalid window ID format")?;

        let state = self.state.lock().unwrap();
        
        if let Some(toplevel) = state.toplevels.get(&id) {
            if let Some(seat) = &state.seat {
                if toplevel.is_minimized {
                    toplevel.handle.unset_minimized();
                    toplevel.handle.activate(seat);
                } else if toplevel.is_activated {
                    toplevel.handle.set_minimized();
                } else {
                    toplevel.handle.activate(seat);
                }
            } else {
                if toplevel.is_minimized {
                     toplevel.handle.unset_minimized();
                } else {
                     toplevel.handle.set_minimized();
                }
                log::warn!("No seat found, window activation might fail");
            }
            // Flush commands
            let _ = self.conn.flush();
            return Ok(());
        }
        
        Err("Window not found".into())
    }
}

// Implement Dispatch for the registry logic
impl Dispatch<wl_registry::WlRegistry, ()> for AppState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<AppState>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            match interface.as_str() {
                "zwlr_foreign_toplevel_manager_v1" => {
                    if version >= 1 {
                        let manager = registry.bind::<ZwlrForeignToplevelManagerV1, _, _>(
                            name, 
                            1.min(version), 
                            qh, 
                            ()
                        );
                        state.manager = Some(manager);
                        log::info!("Found wlr-foreign-toplevel-management protocol");
                    }
                }
                "wl_seat" => {
                    if version >= 1 {
                        let seat = registry.bind::<wl_seat::WlSeat, _, _>(
                            name,
                            1.min(version),
                            qh,
                            ()
                        );
                        state.seat = Some(seat);
                    }
                }
                _ => {}
            }
        }
    }
}

// Implement Dispatch for manager
impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppState>,
    ) {
        match event {
            zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } => {
                let id = toplevel.id().protocol_id();
                let info = ToplevelInfo::new(toplevel);
                state.toplevels.insert(id, info);
            }
            zwlr_foreign_toplevel_manager_v1::Event::Finished => {
                log::info!("Foreign toplevel manager finished");
            }
            _ => {}
        }
    }
}

// Implement Dispatch for handles
impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for AppState {
    fn event(
        state: &mut Self,
        handle: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppState>,
    ) {
        let id = handle.id().protocol_id();
        let mut changed = false;
        
        if let Some(toplevel_info) = state.toplevels.get_mut(&id) {
            match event {
                zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                    toplevel_info.title = title;
                    changed = true;
                }
                zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                    toplevel_info.app_id = app_id;
                    changed = true;
                }
                zwlr_foreign_toplevel_handle_v1::Event::State { state: window_state } => {
                    // Parse the state array
                    let states: Vec<u32> = window_state
                        .chunks_exact(4)
                        .map(|chunk| u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                        .collect();

                    // Old states
                    let old_act = toplevel_info.is_activated;
                    let old_min = toplevel_info.is_minimized;

                    // Reset
                    toplevel_info.is_maximized = false;
                    toplevel_info.is_minimized = false;
                    toplevel_info.is_activated = false;
                    toplevel_info.is_fullscreen = false;

                    // Set new
                    for state_value in states {
                        match state_value {
                            0 => toplevel_info.is_maximized = true,
                            1 => toplevel_info.is_minimized = true,
                            2 => toplevel_info.is_activated = true,
                            3 => toplevel_info.is_fullscreen = true,
                            _ => {}
                        }
                    }
                    
                    if old_act != toplevel_info.is_activated || old_min != toplevel_info.is_minimized {
                         changed = true;
                    }
                }
                zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                    state.toplevels.remove(&id);
                    changed = true;
                }
                zwlr_foreign_toplevel_handle_v1::Event::Done => {
                }
                _ => {}
            }
        }
        
        if changed {
             if let Some(sender) = &state.event_sender {
                 let _ = sender.send(());
             }
        }
    }
}

// Seat stubs
impl Dispatch<wl_seat::WlSeat, ()> for AppState {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppState>,
    ) {}
}

impl Dispatch<wl_output::WlOutput, ()> for AppState {
    fn event(
        _: &mut Self,
        _: &wl_output::WlOutput,
        _: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppState>,
    ) {}
}