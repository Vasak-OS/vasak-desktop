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
use crate::audio_input::{metadata_name, parse_audio_props, volume_percent, BatchTracker, PwNode};
use crate::logger::{log_debug, log_error, log_info};
use crate::structs::VolumeInfo;
use serde_json::Value;
use std::collections::BTreeMap;
use tokio::sync::{oneshot, watch};

#[cfg(feature = "pipewire-native")]
use std::sync::Arc;

/// Error type for PipeWire connection failures.
#[derive(Debug, thiserror::Error)]
pub enum PipeWireError {
    /// Only `PipeWireMonitor::connect` raises this, and it is compiled out
    /// unless the feature is on — hence the gate, or the default build warns
    /// about a variant nothing can construct.
    #[cfg(feature = "pipewire-native")]
    #[error("PipeWire initialization failed: {0}")]
    InitFailed(String),

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
    fn run_loop(state_tx: watch::Sender<VolumeInfo>, shutdown: Arc<std::sync::atomic::AtomicBool>) {
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
                log_error(&format!("PipeWireMonitor: failed to create context: {}", e));
                return;
            }
        };

        let core = match context.connect(None) {
            Ok(c) => c,
            Err(e) => {
                log_error(&format!("PipeWireMonitor: failed to connect core: {}", e));
                return;
            }
        };

        let registry = core
            .get_registry()
            .expect("PipeWire: failed to get registry");

        let state_tx_clone = state_tx.clone();

        // Store bound node proxies and param listeners so they stay alive.
        let bound_nodes = std::sync::Mutex::new(Vec::new());

        // Raw pointer for registry access inside the 'static closure.
        // SAFETY: registry lives on run_loop's stack and is dropped after the
        // main loop exits. The closure only fires during mainloop.iterate()
        // which is called inside run_loop().
        let registry_ptr: *const pipewire::registry::Registry = &registry;

        // Listen for registry global events to find and bind audio sinks
        let _registry_listener = registry
            .add_listener_local()
            .global(move |global| {
                if let Some(props) = global.props {
                    let media_class = props.get("media.class").unwrap_or("");

                    if media_class == "Audio/Sink" {
                        log_debug(&format!(
                            "PipeWireMonitor: found and binding sink node id={}",
                            global.id
                        ));

                        // SAFETY: registry lives on run_loop's stack, outlives this closure.
                        let registry = unsafe { &*registry_ptr };

                        if let Ok(node) = registry.bind::<pipewire::node::Node>(global) {
                            use pipewire::spa::{
                                param::ParamType, pod::deserialize::PodDeserializer, pod::Value,
                                sys,
                            };

                            let tx = state_tx_clone.clone();

                            // Subscribe to Props parameter updates on this node
                            node.subscribe_params(&[ParamType::Props]);

                            // Listen for Props param events containing volume/mute
                            let listener = node
                                .add_listener_local()
                                .param(move |_seq, id, _index, _next, param| {
                                    if id == ParamType::Props {
                                        if let Some(pod) = param {
                                            if let Ok((_, Value::Object(obj))) =
                                                PodDeserializer::deserialize_any_from(
                                                    pod.as_bytes(),
                                                )
                                            {
                                                let mut volume: Option<f64> = None;
                                                let mut muted: Option<bool> = None;

                                                for prop in &obj.properties {
                                                    match prop.key {
                                                        sys::SPA_PROP_volume => {
                                                            if let Value::Float(vol) = prop.value {
                                                                volume = Some(vol as f64);
                                                            }
                                                        }
                                                        sys::SPA_PROP_mute => {
                                                            if let Value::Bool(m) = prop.value {
                                                                muted = Some(m);
                                                            }
                                                        }
                                                        _ => {}
                                                    }
                                                }

                                                if let Some(vol) = volume {
                                                    let pct = (vol * 100.0).round() as i64;
                                                    let info = VolumeInfo {
                                                        current: pct.clamp(0, 100),
                                                        min: 0,
                                                        max: 100,
                                                        is_muted: muted.unwrap_or(false),
                                                    };
                                                    let _ = tx.send(info);
                                                }
                                            }
                                        }
                                    }
                                })
                                .register();

                            // Keep node and listener alive for the duration of the loop
                            // Se recupera el guard: un envenenamiento acá dejaba el monitor de
                            // audio muerto para el resto de la sesión, y lo que se
                            // guarda es una lista completa, no algo a medio escribir.
                            bound_nodes
                                .lock()
                                .unwrap_or_else(|envenenado| envenenado.into_inner())
                                .push((node, listener));
                        } else {
                            log_error(&format!(
                                "PipeWireMonitor: failed to bind sink node id={}",
                                global.id
                            ));
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
    /// Only returns `Ok` after the first pw-dump process has spawned successfully.
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
        let (ready_tx, ready_rx) = oneshot::channel();

        let task = tokio::spawn(async move {
            Self::monitor_loop(state_tx, Some(ready_tx)).await;
        });

        // Wait for the first pw-dump spawn to succeed or fail
        match ready_rx.await {
            Ok(Ok(())) => {
                log_info("PwDumpMonitor: started event-driven monitoring via pw-dump");
                Ok(Self {
                    state_rx,
                    _task: task,
                })
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(PipeWireError::PwDumpNotAvailable(
                "pw-dump monitor task panicked".into(),
            )),
        }
    }

    /// Main monitoring loop that reads pw-dump JSON output.
    /// `ready` signals the first spawn result back to `connect()`.
    async fn monitor_loop(
        state_tx: watch::Sender<VolumeInfo>,
        mut ready: Option<oneshot::Sender<Result<(), PipeWireError>>>,
    ) {
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
                Ok(c) => {
                    // Signal first successful spawn
                    if let Some(ready) = ready.take() {
                        let _ = ready.send(Ok(()));
                    }
                    c
                }
                Err(e) => {
                    log_error(&format!("PwDumpMonitor: failed to spawn pw-dump: {}", e));
                    // Signal first spawn failure
                    if let Some(ready) = ready.take() {
                        let _ = ready.send(Err(PipeWireError::PwDumpNotAvailable(e.to_string())));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => {
                    log_error("PwDumpMonitor: no stdout from pw-dump");
                    if let Some(ready) = ready.take() {
                        let _ = ready.send(Err(PipeWireError::PwDumpNotAvailable(
                            "no stdout from pw-dump".into(),
                        )));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            // Un `pw-dump` nuevo vuelve a mandar el volcado entero: lo que se
            // sabía del micrófono y del sink por omisión puede estar viejo.
            crate::audio_input::reset();
            let mut tracker = OutputTracker::default();
            // Lo último que se mandó por el canal, para no despertar al applet
            // cuando un sink ajeno se re-emite sin tocar el por omisión.
            let mut last_sent: Option<VolumeInfo> = None;

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
                    // Se lee una sola vez y lo usan el volumen y el perfil de
                    // los dispositivos Bluetooth (`bluetooth_audio_profile`),
                    // que sale del mismo flujo sin sumar procesos.
                    if let Ok(batch) = serde_json::from_str::<Value>(&json_buffer) {
                        crate::bluetooth_audio_profile::ingest(&batch);
                        // Y el micrófono (vasak-desktop#182), del mismo flujo.
                        crate::audio_input::ingest(&batch);
                        // El volumen de salida: sólo el sink por omisión, y sólo
                        // cuando cambió. Un sink ajeno re-emitido (cambio de
                        // pestaña, Discord) ya no mueve nada (vasak-desktop#231).
                        tracker.ingest(&batch);
                        if let Some(volume_info) = tracker.default_volume() {
                            if last_sent.as_ref() != Some(&volume_info) {
                                last_sent = Some(volume_info.clone());
                                let _ = state_tx.send(volume_info);
                            }
                        }
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
}

// ─────────────────────────────────────────────────────────────────────────────
// OutputTracker - el sink de salida por omisión a través de las tandas de pw-dump
// ─────────────────────────────────────────────────────────────────────────────

/// Un sink de salida, tal como llegó en la última tanda.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SinkNode {
    name: String,
    /// El volumen lineal (0.0–1.0, puede pasar de 1), ya sin la escala cúbica.
    volume: f64,
    muted: bool,
}

/// Sigue el sink de salida por omisión a lo largo de las tandas de
/// `pw-dump --monitor`.
///
/// `pw-dump --monitor` manda en cada tanda **sólo lo que cambió**, no el
/// volcado entero. En un equipo con varios sinks (analógico + HDMI, el monitor
/// de una salida) PipeWire re-emite un sink que **no** es el por omisión al
/// cambiar de pestaña o al usar Discord, con su volumen crudo (1.0 = 100 %).
/// Reportar el primer `Audio/Sink` de cada tanda mostraba ese volumen ajeno
/// como si fuera el del sistema, y aparecía un OSD de «Volumen: 100 %» espurio
/// (vasak-desktop#231). Por eso se recuerda el volumen de cada sink y cuál es
/// el por omisión (`default.audio.sink`, con `default.configured.audio.sink`
/// de respaldo), y se reporta **sólo** el por omisión. Es el mismo patrón que
/// `audio_input` usa para el micrófono.
#[derive(Debug, Default)]
struct OutputTracker {
    /// Volumen y silencio de cada `Audio/Sink`, por su id.
    sinks: BTreeMap<u64, SinkNode>,
    /// El nombre del sink por omisión.
    default_sink: Option<String>,
}

impl PwNode for SinkNode {
    fn set_level(&mut self, volume: Option<f64>, muted: Option<bool>) {
        self.volume = volume.unwrap_or(self.volume);
        self.muted = muted.unwrap_or(self.muted);
    }
}

impl BatchTracker for OutputTracker {
    type Node = SinkNode;

    fn nodes_mut(&mut self) -> &mut BTreeMap<u64, SinkNode> {
        &mut self.sinks
    }

    fn parse_node(object: &Value) -> Option<SinkNode> {
        parse_sink(object)
    }

    fn ingest_metadata(&mut self, object: &Value) {
        let name = object
            .get("props")
            .and_then(|props| props.get("metadata.name"))
            .and_then(Value::as_str);
        if name != Some("default") {
            return;
        }
        let Some(entries) = object.get("metadata").and_then(Value::as_array) else {
            return;
        };
        for entry in entries {
            match entry.get("key").and_then(Value::as_str) {
                // El que eligió WirePlumber; manda sobre el configurado.
                Some("default.audio.sink") => {
                    self.default_sink = entry.get("value").and_then(metadata_name);
                }
                // De respaldo, por si no viniera el elegido en esta tanda.
                Some("default.configured.audio.sink") if self.default_sink.is_none() => {
                    self.default_sink = entry.get("value").and_then(metadata_name);
                }
                _ => {}
            }
        }
    }
}

impl OutputTracker {
    /// Incorpora una tanda de `pw-dump` (un arreglo de objetos).
    fn ingest(&mut self, batch: &Value) {
        self.ingest_batch(batch);
    }

    /// El volumen y el silencio del sink por omisión, o `None` si todavía no se
    /// sabe cuál es o no llegó su volumen.
    fn default_volume(&self) -> Option<VolumeInfo> {
        let default = self.default_sink.as_deref()?;
        let node = self.sinks.values().find(|node| node.name == default)?;
        Some(VolumeInfo {
            current: volume_percent(node.volume),
            min: 0,
            max: 100,
            is_muted: node.muted,
        })
    }
}

/// Un sink de salida, o `None` si el nodo no es un `Audio/Sink`.
fn parse_sink(object: &Value) -> Option<SinkNode> {
    let info = object.get("info")?;
    let props = info.get("props")?;
    let class = props.get("media.class").and_then(Value::as_str)?;
    if class != "Audio/Sink" {
        return None;
    }
    let name = props.get("node.name").and_then(Value::as_str)?.to_string();
    let (volume, muted) = parse_audio_props(info).unwrap_or((None, None));
    Some(SinkNode {
        name,
        volume: volume.unwrap_or(0.0),
        muted: muted.unwrap_or(false),
    })
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

#[cfg(test)]
mod tests {
    //! El `OutputTracker`: que siga el sink por omisión a través de las tandas
    //! incrementales de `pw-dump --monitor` y reporte **sólo** ese, nunca un
    //! sink ajeno re-emitido (el OSD de «Volumen: 100 %» espurio de
    //! vasak-desktop#231). Mismo formato de `pw-dump` que las pruebas de
    //! `audio_input`.

    use super::*;
    use serde_json::json;

    const SPEAKERS: &str = "alsa_output.pci-0000_00_1f.3.analog-stereo";
    const HDMI: &str = "alsa_output.pci-0000_01_00.1.hdmi-stereo";

    /// Un `Audio/Sink` como lo manda `pw-dump`. `cubic` es el volumen en la
    /// escala cúbica de PipeWire (1.0 → 100 %, 0.064 → 40 %, 0.216 → 60 %).
    fn sink(id: u64, name: &str, cubic: f64, mute: bool) -> Value {
        json!({
            "id": id,
            "type": "PipeWire:Interface:Node",
            "info": {
                "props": { "media.class": "Audio/Sink", "node.name": name },
                "params": {
                    "Props": [
                        { "volume": 1.0, "mute": mute, "channelVolumes": [cubic] },
                        { "params": ["monitor.channel-volumes", false] }
                    ]
                }
            }
        })
    }

    /// El `Metadata` `default` con los pares que se le pasen (clave → nombre del
    /// sink), en ese orden.
    fn metadata(pairs: &[(&str, &str)]) -> Value {
        let entries: Vec<Value> = pairs
            .iter()
            .map(|(key, name)| {
                json!({ "subject": 0, "key": key, "type": "Spa:String:JSON",
                        "value": { "name": name } })
            })
            .collect();
        json!({
            "id": 39,
            "type": "PipeWire:Interface:Metadata",
            "props": { "metadata.name": "default" },
            "metadata": entries
        })
    }

    fn vol(current: i64, muted: bool) -> VolumeInfo {
        VolumeInfo {
            current,
            min: 0,
            max: 100,
            is_muted: muted,
        }
    }

    /// El volcado inicial: dos sinks —la placa al 40 % (por omisión) y el HDMI
    /// al 100 % (ajeno)— y el metadato que elige la placa. El HDMI lleva el id
    /// más alto a propósito: así «el primer/último sink cualquiera» (el bug)
    /// cae sobre él y se distingue del por omisión.
    fn tracker_with_default() -> OutputTracker {
        let mut tracker = OutputTracker::default();
        tracker.ingest(&json!([
            sink(40, SPEAKERS, 0.064, false),
            sink(55, HDMI, 1.0, false),
            metadata(&[("default.audio.sink", SPEAKERS)]),
        ]));
        tracker
    }

    #[test]
    fn resuelve_el_sink_por_omision_desde_default_audio_sink() {
        assert_eq!(
            tracker_with_default().default_volume(),
            Some(vol(40, false))
        );
    }

    #[test]
    fn una_tanda_con_un_sink_ajeno_al_100_no_cambia_el_volumen_por_omision() {
        let mut tracker = tracker_with_default();
        // `pw-dump --monitor` re-emite SÓLO el sink ajeno (HDMI) a su volumen
        // crudo (100 %) al cambiar de pestaña o con Discord: eso no es el
        // volumen del sistema y no debe moverlo (vasak-desktop#231). Es la que
        // reproduce el «100 % espurio»: el parser viejo devolvía ese sink.
        tracker.ingest(&json!([sink(55, HDMI, 1.0, false)]));
        assert_eq!(tracker.default_volume(), Some(vol(40, false)));
    }

    #[test]
    fn un_cambio_real_del_sink_por_omision_se_refleja() {
        let mut tracker = tracker_with_default();
        // La placa (por omisión) baja a 60 % y se silencia: eso sí se reporta.
        tracker.ingest(&json!([sink(40, SPEAKERS, 0.216, true)]));
        assert_eq!(tracker.default_volume(), Some(vol(60, true)));
    }

    #[test]
    fn un_cambio_de_nivel_sin_propiedades_actualiza_el_sink_por_omision() {
        let mut tracker = tracker_with_default();
        // Una tanda que sólo trae `params` (sin `props`): es una actualización
        // de volumen de un sink ya conocido, no un nodo nuevo.
        tracker.ingest(&json!([{
            "id": 40,
            "type": "PipeWire:Interface:Node",
            "info": { "params": { "Props": [{ "mute": false, "channelVolumes": [0.216] }] } }
        }]));
        assert_eq!(tracker.default_volume(), Some(vol(60, false)));
    }

    #[test]
    fn default_configured_audio_sink_es_el_respaldo_cuando_no_vino_el_elegido() {
        let mut tracker = OutputTracker::default();
        tracker.ingest(&json!([
            sink(40, SPEAKERS, 0.064, false),
            sink(55, HDMI, 1.0, false),
            metadata(&[("default.configured.audio.sink", SPEAKERS)]),
        ]));
        assert_eq!(tracker.default_volume(), Some(vol(40, false)));
    }

    #[test]
    fn el_sink_elegido_manda_sobre_el_configurado() {
        let mut tracker = OutputTracker::default();
        // El configurado apunta al HDMI, pero WirePlumber eligió la placa: gana
        // `default.audio.sink` aunque venga después en la tanda.
        tracker.ingest(&json!([
            sink(40, SPEAKERS, 0.064, false),
            sink(55, HDMI, 1.0, false),
            metadata(&[
                ("default.configured.audio.sink", HDMI),
                ("default.audio.sink", SPEAKERS),
            ]),
        ]));
        assert_eq!(tracker.default_volume(), Some(vol(40, false)));
    }

    #[test]
    fn sin_saber_cual_es_el_por_omision_no_hay_volumen() {
        let mut tracker = OutputTracker::default();
        tracker.ingest(&json!([sink(55, HDMI, 1.0, false)]));
        assert_eq!(tracker.default_volume(), None);
    }

    #[test]
    fn el_sink_por_omision_que_se_va_deja_de_tener_volumen() {
        let mut tracker = tracker_with_default();
        tracker.ingest(&json!([{ "id": 40, "info": null }]));
        assert_eq!(tracker.default_volume(), None);
    }
}
