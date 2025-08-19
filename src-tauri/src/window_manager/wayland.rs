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

// Import KDE Plasma protocols
use wayland_protocols_plasma::plasma_window_management::client::{
    org_kde_plasma_window::{self, OrgKdePlasmaWindow},
    org_kde_plasma_window_management::{self, OrgKdePlasmaWindowManagement},
};

#[derive(Debug, Clone)]
enum WindowType {
    Wlr(ToplevelInfo),
    Kde(KdeToplevelInfo),
}

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

#[derive(Debug, Clone)]
struct KdeToplevelInfo {
    handle: OrgKdePlasmaWindow,
    title: String,
    app_id: String,
    is_maximized: bool,
    is_minimized: bool,
    is_activated: bool,
    is_fullscreen: bool,
    desktop: i32,
}

impl ToplevelInfo {
    fn new_wlr(handle: ZwlrForeignToplevelHandleV1) -> Self {
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
            demands_attention: None, // Wayland doesn't have direct equivalent
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
        
        let app_id_lower = self.app_id.to_lowercase();
        !skip_apps.iter().any(|app| app_id_lower.contains(app))
    }
}

impl KdeToplevelInfo {
    fn new_kde(handle: OrgKdePlasmaWindow) -> Self {
        Self {
            handle,
            title: String::new(),
            app_id: String::new(),
            is_maximized: false,
            is_minimized: false,
            is_activated: false,
            is_fullscreen: false,
            desktop: 0,
        }
    }

    fn to_window_info(&self, id: &str) -> WindowInfo {
        WindowInfo {
            id: id.to_string(),
            title: self.title.clone(),
            is_minimized: self.is_minimized,
            icon: self.app_id.clone(),
            demands_attention: None, // KDE doesn't have direct equivalent
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
        
        let app_id_lower = self.app_id.to_lowercase();
        !skip_apps.iter().any(|app| app_id_lower.contains(app))
    }
}

#[derive(Debug, Clone, Copy)]
enum ProtocolType {
    Wlr,
    #[allow(dead_code)]
    Kde, // For future implementation
}

struct AppState {
    wlr_toplevels: HashMap<u32, ToplevelInfo>,
    kde_toplevels: HashMap<u32, KdeToplevelInfo>,
    wlr_manager: Option<ZwlrForeignToplevelManagerV1>,
    kde_manager: Option<OrgKdePlasmaWindowManagement>,
    seat: Option<wl_seat::WlSeat>,
    event_sender: Option<Sender<()>>,
    protocol_type: Option<ProtocolType>,
}

impl AppState {
    fn new() -> Self {
        Self {
            wlr_toplevels: HashMap::new(),
            kde_toplevels: HashMap::new(),
            wlr_manager: None,
            kde_manager: None,
            seat: None,
            event_sender: None,
            protocol_type: None,
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
        
        // Check which protocol is available
        let state_guard = self.state.lock().unwrap();
        log::info!("Checking available protocols...");
        log::info!("WLR manager available: {}", state_guard.wlr_manager.is_some());
        log::info!("KDE manager available: {}", state_guard.kde_manager.is_some());
        
        if state_guard.wlr_manager.is_some() {
            log::info!("Using wlr-foreign-toplevel-management protocol");
            return Ok(());
        }
        
        if state_guard.kde_manager.is_some() {
            log::info!("Using KDE Plasma window management protocol");
            return Ok(());
        }
        
        drop(state_guard);
        return Err("No supported window management protocol available (tried wlr-foreign-toplevel-management and KDE Plasma protocols).".into());
    }
}

impl WindowManagerBackend for WaylandManager {
    fn get_window_list(&mut self) -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>> {
        // Dispatch any pending events first
        if let Err(e) = self.event_queue.blocking_dispatch(&mut *self.state.lock().unwrap()) {
            log::warn!("Failed to dispatch events: {}", e);
            return Ok(Vec::new());
        }

        let state = self.state.lock().unwrap();
        let mut windows: Vec<WindowInfo> = Vec::new();
        
        // Add wlr windows
        for (id, toplevel) in &state.wlr_toplevels {
            if toplevel.should_show() {
                windows.push(toplevel.to_window_info(&id.to_string()));
            }
        }
        
        // Add KDE windows  
        for (id, toplevel) in &state.kde_toplevels {
            if toplevel.should_show() {
                windows.push(toplevel.to_window_info(&id.to_string()));
            }
        }

        Ok(windows)
    }

    fn setup_event_monitoring(&mut self, tx: Sender<()>) -> Result<(), Box<dyn std::error::Error>> {
        match self.setup_protocol_bindings() {
            Ok(_) => {
                let protocol_name = {
                    let state = self.state.lock().unwrap();
                    match state.protocol_type {
                        Some(ProtocolType::Wlr) => "wlr-foreign-toplevel-management",
                        Some(ProtocolType::Kde) => "kde-plasma-window-management",
                        None => "unknown",
                    }
                };
                log::info!("Wayland window management initialized successfully using {}", protocol_name);
            }
            Err(e) => {
                log::error!("Failed to initialize Wayland protocols: {}", e);
                log::warn!("Window management monitoring will not be available");
                log::info!("Note: If you're using KDE/Plasma, support is being developed");
                return Err(e);
            }
        }
        
        // Store the sender for events
        {
            let mut state = self.state.lock().unwrap();
            state.event_sender = Some(tx);
        }

        let state_clone = Arc::clone(&self.state);
        let mut event_queue = self.conn.new_event_queue::<AppState>();
        
        std::thread::spawn(move || {
            loop {
                match event_queue.blocking_dispatch(&mut *state_clone.lock().unwrap()) {
                    Ok(_) => {
                        // Notify of window list changes
                        if let Some(sender) = &state_clone.lock().unwrap().event_sender {
                            let _ = sender.send(());
                        }
                    }
                    Err(e) => {
                        log::error!("Error in Wayland event loop: {}", e);
                        break;
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
        
        // Try wlr first
        if let Some(toplevel) = state.wlr_toplevels.get(&id) {
            if let Some(seat) = &state.seat {
                if toplevel.is_minimized {
                    toplevel.handle.unset_minimized();
                    toplevel.handle.activate(seat);
                } else if toplevel.is_activated {
                    toplevel.handle.set_minimized();
                } else {
                    toplevel.handle.activate(seat);
                }
            }
            return Ok(());
        }
        
        // Try KDE
        if let Some(_toplevel) = state.kde_toplevels.get(&id) {
            // TODO: Implement KDE specific window operations
            log::info!("KDE window toggle not yet implemented for window {}", id);
            return Ok(());
        }
        
        Err("Window not found".into())
    }
}

// Implement Dispatch for the registry to bind protocols
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
            log::info!("Found Wayland global: {} (version {})", interface, version);
            match interface.as_str() {
                "zwlr_foreign_toplevel_manager_v1" => {
                    if version >= 1 {
                        let manager = registry.bind::<ZwlrForeignToplevelManagerV1, _, _>(
                            name, 
                            1.min(version), 
                            qh, 
                            ()
                        );
                        state.wlr_manager = Some(manager);
                        state.protocol_type = Some(ProtocolType::Wlr);
                        log::info!("Found wlr-foreign-toplevel-management protocol");
                    }
                }
                "org_kde_plasma_window_management" => {
                    if version >= 1 {
                        let manager = registry.bind::<OrgKdePlasmaWindowManagement, _, _>(
                            name,
                            1.min(version),
                            qh,
                            ()
                        );
                        state.kde_manager = Some(manager);
                        state.protocol_type = Some(ProtocolType::Kde);
                        log::info!("Found and bound KDE Plasma window management protocol");
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

// Implement Dispatch for the wlr foreign toplevel manager
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
                let info = ToplevelInfo::new_wlr(toplevel);
                state.wlr_toplevels.insert(id, info);
            }
            zwlr_foreign_toplevel_manager_v1::Event::Finished => {
                log::info!("Foreign toplevel manager finished");
            }
            _ => {}
        }
    }
}

// Implement Dispatch for individual wlr toplevel handles
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
        
        if let Some(toplevel_info) = state.wlr_toplevels.get_mut(&id) {
            match event {
                zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                    toplevel_info.title = title;
                }
                zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                    toplevel_info.app_id = app_id;
                }
                zwlr_foreign_toplevel_handle_v1::Event::State { state: window_state } => {
                    // Parse the state array
                    let states: Vec<u32> = window_state
                        .chunks_exact(4)
                        .map(|chunk| u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                        .collect();

                    // Reset all states
                    toplevel_info.is_maximized = false;
                    toplevel_info.is_minimized = false;
                    toplevel_info.is_activated = false;
                    toplevel_info.is_fullscreen = false;

                    // Set current states using the enum values from the protocol
                    for state_value in states {
                        match state_value {
                            0 => toplevel_info.is_maximized = true,  // maximized
                            1 => toplevel_info.is_minimized = true,  // minimized  
                            2 => toplevel_info.is_activated = true,  // activated
                            3 => toplevel_info.is_fullscreen = true, // fullscreen
                            _ => {}
                        }
                    }
                }
                zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                    state.wlr_toplevels.remove(&id);
                }
                zwlr_foreign_toplevel_handle_v1::Event::Done => {
                    // All properties have been sent, can notify of changes
                    if let Some(sender) = &state.event_sender {
                        let _ = sender.send(());
                    }
                }
                _ => {} // Handle other events as needed
            }
        }
    }
}

// TODO: Add KDE Plasma protocol implementations here

// Implement Dispatch for wl_seat (required but we don't need to handle events)
impl Dispatch<wl_seat::WlSeat, ()> for AppState {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppState>,
    ) {
        // We don't need to handle seat events for this use case
    }
}

// Implement Dispatch for wl_output (may be needed for output events)
impl Dispatch<wl_output::WlOutput, ()> for AppState {
    fn event(
        _: &mut Self,
        _: &wl_output::WlOutput,
        _: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppState>,
    ) {
        // We don't need to handle output events for this use case
    }
}

// Implement Dispatch for KDE Plasma window management
impl Dispatch<OrgKdePlasmaWindowManagement, ()> for AppState {
    fn event(
        _state: &mut Self,
        _: &OrgKdePlasmaWindowManagement,
        event: org_kde_plasma_window_management::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppState>,
    ) {
        match event {
            org_kde_plasma_window_management::Event::Window { id } => {
                // TODO: Properly handle KDE window creation
                log::info!("KDE window management event for window id: {}", id);
            }
            _ => {}
        }
    }
}

// Implement Dispatch for KDE Plasma window
impl Dispatch<OrgKdePlasmaWindow, ()> for AppState {
    fn event(
        state: &mut Self,
        handle: &OrgKdePlasmaWindow,
        event: org_kde_plasma_window::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppState>,
    ) {
        let id = handle.id().protocol_id();
        
        if let Some(window_info) = state.kde_toplevels.get_mut(&id) {
            match event {
                org_kde_plasma_window::Event::TitleChanged { title } => {
                    window_info.title = title;
                }
                org_kde_plasma_window::Event::AppIdChanged { app_id } => {
                    window_info.app_id = app_id;
                }
                org_kde_plasma_window::Event::StateChanged { flags } => {
                    // TODO: Parse KDE state flags properly
                    window_info.is_minimized = flags & 1 != 0; // Assuming bit 0 is minimized
                    window_info.is_activated = flags & 2 != 0; // Assuming bit 1 is activated  
                    window_info.is_maximized = flags & 4 != 0; // Assuming bit 2 is maximized
                    window_info.is_fullscreen = flags & 8 != 0; // Assuming bit 3 is fullscreen
                }
                org_kde_plasma_window::Event::VirtualDesktopChanged { number } => {
                    window_info.desktop = number;
                }
                org_kde_plasma_window::Event::Unmapped => {
                    state.kde_toplevels.remove(&id);
                    log::info!("KDE window {} unmapped", id);
                }
                _ => {
                    log::debug!("Unhandled KDE window event: {:?}", event);
                }
            }
            
            // Notify of changes
            if let Some(sender) = &state.event_sender {
                let _ = sender.send(());
            }
        }
    }
}