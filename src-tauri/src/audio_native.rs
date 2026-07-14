//! Native PipeWire audio monitoring with fallback to pactl polling.
//!
//! This module provides audio monitoring implementations:
//!
//! 1. `PipeWireMonitor` (feature = "pipewire-native") - Uses pipewire-rs to subscribe
//!    to volume/mute changes on the default audio sink via PipeWire's native event loop.
//!    Because libpipewire requires its own main loop thread, the PipeWire event loop runs
//!    on a dedicated `std::thread` and bridges updates to tokio via a `watch` channel.
//!
//! 2. `PwDumpMonitor` (default) - Uses `pw-dump --monitor` as a streaming process that
//!    emits JSON events on stdout when PipeWire objects change. This is significantly
//!    better than pactl polling because it's event-driven without spawning new processes.
//!
//! 3. `PactlFallback` - Wraps the existing `audio::get_volume()` function in a simple
//!    tokio interval loop at 2000ms, used when PipeWire tools are also unavailable.

use crate::audio;
use crate::logger::{log_debug, log_error, log_info};
use crate::structs::VolumeInfo;
use tokio::sync::watch;

#[cfg(feature = "pipewire-native")]
use std::sync::Arc;

/// Error type for PipeWire connection failures.
#[derive(Debug, thiserror::Error)]
pub enum PipeWireError {
    #[error("PipeWire initialization failed: {0}")]
    InitFailed(String),

    #[error("PipeWire main loop creation failed: {0}")]
    MainLoopFailed(String),

    #[error("PipeWire context creation failed: {0}")]
    ContextFailed(String),

    #[error("PipeWire core connection failed: {0}")]
    CoreFailed(String),

    #[error("PipeWire thread panicked")]
    ThreadPanicked,

    #[error("pw-dump process not available: {0}")]
    PwDumpNotAvailable(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// PipeWireMonitor - native pipewire-rs (requires feature "pipewire-native")
// ─────────────────────────────────────────────────────────────────────────────

/// Monitors audio volume/mute state via PipeWire's native event API.
///
/// Spawns a dedicated OS thread running the PipeWire main loop and bridges
/// state changes to the async world via a `tokio::sync::watch` channel.
///
/// Only available when compiled with the `pipewire-native` feature and
/// libpipewire-0.3 development headers are installed.
#[cfg(feature = "pipewire-native")]
pub struct PipeWireMonitor {
    /// Receives the latest VolumeInfo whenever the default sink changes.
    pub state_rx: watch::Receiver<VolumeInfo>,
    /// Handle to signal the PipeWire thread to stop.
    _shutdown: Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(feature = "pipewire-native")]
impl PipeWireMonitor {
    /// Attempts to connect to PipeWire and start monitoring the default sink.
    ///
    /// On success, returns a `PipeWireMonitor` whose `state_rx` will receive
    /// `VolumeInfo` updates whenever the default sink volume or mute state changes.
    pub fn connect() -> Result<Self, PipeWireError> {
        let initial_volume = VolumeInfo {
            current: 0,
            min: 0,
            max: 100,
            is_muted: false,
        };

        let (state_tx, state_rx) = watch::channel(initial_volume);
        let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        // Spawn a dedicated OS thread for PipeWire's main loop.
        // PipeWire's C library cannot run inside tokio's async runtime.
        let builder = std::thread::Builder::new().name("pipewire-monitor".into());
        let handle = builder
            .spawn(move || {
                Self::run_loop(state_tx, shutdown_clone);
            })
            .map_err(|e| PipeWireError::InitFailed(format!("Failed to spawn thread: {}", e)))?;

        // Give the thread a moment to initialize. If it panics immediately
        // (e.g., libpipewire not found), we catch it here.
        std::thread::sleep(std::time::Duration::from_millis(100));

        if handle.is_finished() {
            return Err(PipeWireError::InitFailed(
                "PipeWire thread exited immediately - library may not be available".into(),
            ));
        }

        log_info("PipeWireMonitor: connected successfully, monitoring default sink");

        Ok(Self {
            state_rx,
            _shutdown: shutdown,
        })
    }

    /// Runs the PipeWire main loop on a dedicated thread.
    ///
    /// This function subscribes to the PipeWire registry, finds the default
    /// audio sink, and listens for volume/mute param changes.
    fn run_loop(
        state_tx: watch::Sender<VolumeInfo>,
        shutdown: Arc<std::sync::atomic::AtomicBool>,
    ) {
        use pipewire::prelude::*;

        // Initialize PipeWire (must be called on the thread that runs the loop)
        pipewire::init();

        let mainloop = match pipewire::main_loop::MainLoop::new(None) {
            Ok(ml) => ml,
            Err(e) => {
                log_error(&format!(
                    "PipeWireMonitor: failed to create main loop: {}",
                    e
                ));
                return;
            }
        };

        let context = match pipewire::context::Context::new(&mainloop) {
            Ok(ctx) => ctx,
            Err(e) => {
                log_error(&format!(
                    "PipeWireMonitor: failed to create context: {}",
                    e
                ));
                return;
            }
        };

        let core = match context.connect(None) {
            Ok(c) => c,
            Err(e) => {
                log_error(&format!(
                    "PipeWireMonitor: failed to connect core: {}",
                    e
                ));
                return;
            }
        };

        let registry = core.get_registry().expect("PipeWire: failed to get registry");

        let state_tx_clone = state_tx.clone();

        // Listen for registry global events to find audio sinks
        let _registry_listener = registry
            .add_listener_local()
            .global(move |global| {
                if let Some(props) = global.props {
                    let media_class = props.get("media.class").unwrap_or("");

                    if media_class == "Audio/Sink" {
                        log_debug(&format!(
                            "PipeWireMonitor: found sink node id={}",
                            global.id
                        ));

                        // Extract volume info from properties if available
                        if let Some(vol_str) = props.get("volume.level") {
                            if let Ok(vol_f) = vol_str.parse::<f64>() {
                                let volume_pct = (vol_f * 100.0).round() as i64;
                                let is_muted = props
                                    .get("volume.mute")
                                    .map(|v| v == "1" || v == "true")
                                    .unwrap_or(false);

                                let info = VolumeInfo {
                                    current: volume_pct.clamp(0, 100),
                                    min: 0,
                                    max: 100,
                                    is_muted,
                                };

                                let _ = state_tx_clone.send(info);
                            }
                        }
                    }
                }
            })
            .register();

        // Run the PipeWire main loop (blocks until quit)
        loop {
            if shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            mainloop.iterate(std::time::Duration::from_millis(200));
        }

        log_info("PipeWireMonitor: main loop exited");
    }
}

#[cfg(feature = "pipewire-native")]
impl Drop for PipeWireMonitor {
    fn drop(&mut self) {
        self._shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PwDumpMonitor - uses `pw-dump --monitor` process for event-driven monitoring
// ─────────────────────────────────────────────────────────────────────────────

/// Monitors audio volume/mute state via `pw-dump --monitor` process.
///
/// This spawns a single long-lived `pw-dump --monitor` process that streams
/// JSON events on stdout whenever PipeWire objects change. This is event-driven
/// (no polling) and doesn't require libpipewire headers at compile time.
pub struct PwDumpMonitor {
    /// Receives the latest VolumeInfo from pw-dump events.
    pub state_rx: watch::Receiver<VolumeInfo>,
    /// Handle to the background monitoring task (aborts on drop).
    _task: tokio::task::JoinHandle<()>,
}

impl PwDumpMonitor {
    /// Attempts to start monitoring via `pw-dump --monitor`.
    ///
    /// Returns an error if pw-dump is not found or fails to start.
    pub async fn connect() -> Result<Self, PipeWireError> {
        // First, check if pw-dump is available
        let check = tokio::process::Command::new("which")
            .arg("pw-dump")
            .output()
            .await
            .map_err(|e| PipeWireError::PwDumpNotAvailable(e.to_string()))?;

        if !check.status.success() {
            return Err(PipeWireError::PwDumpNotAvailable(
                "pw-dump not found in PATH".into(),
            ));
        }

        let initial_volume = VolumeInfo {
            current: 0,
            min: 0,
            max: 100,
            is_muted: false,
        };

        let (state_tx, state_rx) = watch::channel(initial_volume);

        let task = tokio::spawn(async move {
            Self::monitor_loop(state_tx).await;
        });

        log_info("PwDumpMonitor: started event-driven monitoring via pw-dump");

        Ok(Self {
            state_rx,
            _task: task,
        })
    }

    /// Main monitoring loop that reads pw-dump JSON output.
    async fn monitor_loop(state_tx: watch::Sender<VolumeInfo>) {
        use tokio::io::{AsyncBufReadExt, BufReader};
        use tokio::process::Command;

        loop {
            // Start pw-dump --monitor --no-colors
            let child = Command::new("pw-dump")
                .args(["--monitor", "--no-colors"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .spawn();

            let mut child = match child {
                Ok(c) => c,
                Err(e) => {
                    log_error(&format!("PwDumpMonitor: failed to spawn pw-dump: {}", e));
                    // Wait before retry
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => {
                    log_error("PwDumpMonitor: no stdout from pw-dump");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut json_buffer = String::new();
            let mut bracket_depth: i32 = 0;

            // Read JSON objects from pw-dump output
            while let Ok(Some(line)) = lines.next_line().await {
                // pw-dump outputs JSON arrays of objects. Track bracket depth
                // to know when we have a complete JSON fragment.
                for ch in line.chars() {
                    match ch {
                        '[' | '{' => bracket_depth += 1,
                        ']' | '}' => bracket_depth -= 1,
                        _ => {}
                    }
                }

                json_buffer.push_str(&line);
                json_buffer.push('\n');

                // When brackets are balanced, we have a complete JSON chunk
                if bracket_depth == 0 && !json_buffer.trim().is_empty() {
                    if let Some(volume_info) = Self::parse_volume_from_json(&json_buffer) {
                        let _ = state_tx.send(volume_info);
                    }
                    json_buffer.clear();
                }
            }

            // pw-dump exited - wait before restarting
            log_info("PwDumpMonitor: pw-dump process exited, restarting in 2s");
            let _ = child.wait().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    /// Parses pw-dump JSON output to extract volume/mute for the default sink.
    ///
    /// pw-dump outputs arrays of PipeWire objects. We look for objects with
    /// type "PipeWire:Interface:Node" and media.class "Audio/Sink" that have
    /// volume parameters.
    fn parse_volume_from_json(json_str: &str) -> Option<VolumeInfo> {
        // Parse as a JSON value
        let value: serde_json::Value = serde_json::from_str(json_str).ok()?;

        // pw-dump outputs an array of objects
        let objects = value.as_array()?;

        for obj in objects {
            // Check if this is a Node with Audio/Sink media class
            let obj_type = obj.get("type")?.as_str()?;
            if obj_type != "PipeWire:Interface:Node" {
                continue;
            }

            let info = obj.get("info")?;
            let props = info.get("props")?;

            let media_class = props.get("media.class").and_then(|v| v.as_str())?;
            if media_class != "Audio/Sink" {
                continue;
            }

            // Look for volume parameters in the params section
            let params = info.get("params")?;

            // Check Props params for volume info
            if let Some(props_array) = params.get("Props").and_then(|v| v.as_array()) {
                for prop_entry in props_array {
                    // Look for channelVolumes and mute in the props
                    let mute = prop_entry.get("mute").and_then(|v| v.as_bool()).unwrap_or(false);

                    if let Some(volumes) =
                        prop_entry.get("channelVolumes").and_then(|v| v.as_array())
                    {
                        // PipeWire volumes are cubic: actual_volume = pw_vol^3
                        // Convert to percentage: pct = cbrt(pw_vol) * 100
                        // Actually PipeWire uses linear 0.0..1.0 that maps to the
                        // cubic scale internally. The raw value is the linear volume.
                        if let Some(first_vol) = volumes.first().and_then(|v| v.as_f64()) {
                            // Convert PipeWire linear volume (0.0-1.0+) to percentage
                            // PipeWire uses cubic scale: percentage = cbrt(linear) * 100
                            let pct = (first_vol.cbrt() * 100.0).round() as i64;

                            return Some(VolumeInfo {
                                current: pct.clamp(0, 150), // Can go above 100%
                                min: 0,
                                max: 100,
                                is_muted: mute,
                            });
                        }
                    }

                    // Alternative: look for volume as a single float
                    if let Some(vol) = prop_entry.get("volume").and_then(|v| v.as_f64()) {
                        let pct = (vol.cbrt() * 100.0).round() as i64;
                        return Some(VolumeInfo {
                            current: pct.clamp(0, 150),
                            min: 0,
                            max: 100,
                            is_muted: mute,
                        });
                    }
                }
            }
        }

        None
    }
}

impl Drop for PwDumpMonitor {
    fn drop(&mut self) {
        self._task.abort();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PactlFallback - wraps existing pactl-based polling
// ─────────────────────────────────────────────────────────────────────────────

/// Fallback audio monitor that uses `pactl` commands at a fixed 2000ms interval.
///
/// This is used when PipeWire native monitoring and pw-dump are both unavailable.
/// It wraps the existing `audio::get_volume()` function and sends updates through
/// a watch channel.
pub struct PactlFallback {
    /// Receives the latest VolumeInfo from polling.
    pub state_rx: watch::Receiver<VolumeInfo>,
    /// Handle to the background polling task.
    _task: tokio::task::JoinHandle<()>,
}

/// Interval for pactl fallback polling in milliseconds.
const PACTL_FALLBACK_POLL_MS: u64 = 2000;

impl PactlFallback {
    /// Starts polling audio volume via `pactl` every 2000ms.
    ///
    /// Returns a `PactlFallback` whose `state_rx` will receive `VolumeInfo`
    /// updates whenever the polled volume differs from the previous reading.
    pub fn start() -> Self {
        let initial_volume = VolumeInfo {
            current: 0,
            min: 0,
            max: 100,
            is_muted: false,
        };

        let (state_tx, state_rx) = watch::channel(initial_volume);

        let task = tokio::spawn(async move {
            Self::poll_loop(state_tx).await;
        });

        log_info("PactlFallback: started polling at 2000ms intervals");

        Self {
            state_rx,
            _task: task,
        }
    }

    /// Polling loop that reads volume via pactl and sends changes through the channel.
    async fn poll_loop(state_tx: watch::Sender<VolumeInfo>) {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_millis(PACTL_FALLBACK_POLL_MS));
        let mut last_volume: Option<VolumeInfo> = None;

        loop {
            interval.tick().await;

            match audio::get_volume() {
                Ok(current) => {
                    let has_changed = match &last_volume {
                        None => true,
                        Some(last) => {
                            last.current != current.current || last.is_muted != current.is_muted
                        }
                    };

                    if has_changed {
                        log_debug(&format!(
                            "PactlFallback: volume changed to {}% muted={}",
                            current.current, current.is_muted
                        ));
                        let _ = state_tx.send(current.clone());
                        last_volume = Some(current);
                    }
                }
                Err(e) => {
                    log_error(&format!("PactlFallback: failed to get volume: {}", e));
                }
            }
        }
    }
}

impl Drop for PactlFallback {
    fn drop(&mut self) {
        self._task.abort();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AudioMonitor - unified interface for the applet
// ─────────────────────────────────────────────────────────────────────────────

/// Unified audio monitor that attempts PipeWire first, falls back to pactl.
///
/// Provides a single `watch::Receiver<VolumeInfo>` regardless of which
/// backend is active. The priority order is:
///
/// 1. Native PipeWire (if compiled with `pipewire-native` feature)
/// 2. pw-dump process monitor (event-driven, no polling)
/// 3. pactl polling fallback (2000ms interval)
pub enum AudioMonitor {
    /// Using PipeWire native event-driven monitoring (requires feature).
    #[cfg(feature = "pipewire-native")]
    PipeWire(PipeWireMonitor),
    /// Using pw-dump process-based event monitoring.
    PwDump(PwDumpMonitor),
    /// Using pactl polling fallback.
    Pactl(PactlFallback),
}

impl AudioMonitor {
    /// Creates an audio monitor using the best available backend.
    ///
    /// Attempts PipeWire native (if feature enabled), then pw-dump, then pactl.
    pub async fn new() -> Self {
        // Try native PipeWire first (if compiled with feature)
        #[cfg(feature = "pipewire-native")]
        {
            match PipeWireMonitor::connect() {
                Ok(pw) => {
                    log_info("AudioMonitor: using PipeWire native monitoring");
                    return AudioMonitor::PipeWire(pw);
                }
                Err(e) => {
                    log_info(&format!(
                        "AudioMonitor: PipeWire native unavailable ({}), trying pw-dump",
                        e
                    ));
                }
            }
        }

        // Try pw-dump process monitor
        match PwDumpMonitor::connect().await {
            Ok(pwd) => {
                log_info("AudioMonitor: using pw-dump event-driven monitoring");
                return AudioMonitor::PwDump(pwd);
            }
            Err(e) => {
                log_info(&format!(
                    "AudioMonitor: pw-dump unavailable ({}), using pactl fallback",
                    e
                ));
            }
        }

        // Fall back to pactl polling
        log_info("AudioMonitor: using pactl polling fallback (2000ms)");
        AudioMonitor::Pactl(PactlFallback::start())
    }

    /// Returns a clone of the watch receiver for volume state.
    pub fn state_rx(&self) -> watch::Receiver<VolumeInfo> {
        match self {
            #[cfg(feature = "pipewire-native")]
            AudioMonitor::PipeWire(pw) => pw.state_rx.clone(),
            AudioMonitor::PwDump(pwd) => pwd.state_rx.clone(),
            AudioMonitor::Pactl(pa) => pa.state_rx.clone(),
        }
    }

    /// Returns true if currently using native PipeWire monitoring.
    pub fn is_native(&self) -> bool {
        #[cfg(feature = "pipewire-native")]
        if matches!(self, AudioMonitor::PipeWire(_)) {
            return true;
        }
        false
    }

    /// Returns true if using event-driven monitoring (native or pw-dump).
    pub fn is_event_driven(&self) -> bool {
        match self {
            #[cfg(feature = "pipewire-native")]
            AudioMonitor::PipeWire(_) => true,
            AudioMonitor::PwDump(_) => true,
            AudioMonitor::Pactl(_) => false,
        }
    }

    /// Returns a description of the active monitoring backend.
    pub fn backend_name(&self) -> &'static str {
        match self {
            #[cfg(feature = "pipewire-native")]
            AudioMonitor::PipeWire(_) => "pipewire-native",
            AudioMonitor::PwDump(_) => "pw-dump",
            AudioMonitor::Pactl(_) => "pactl-polling",
        }
    }
}
