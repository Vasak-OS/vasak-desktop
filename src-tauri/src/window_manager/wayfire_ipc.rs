use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::env;
use std::error::Error;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::RwLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use tokio::sync::{broadcast, Mutex as AsyncMutex, Notify};
use tokio::time::{sleep, Duration, Instant};

/// Reads a number that Wayfire may send as either an integer or a float.
///
/// Wayfire 0.11 moved its geometry to floating point, so coordinates arrive as
/// `0.0` where they used to be `0`. Insisting on an integer made every window
/// list fail to parse — the log filled with "invalid type: floating point
/// `0.0`, expected i64" once a second, and the panel showed no open windows at
/// all because it never received one.
fn number_as_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Number(number) => number
            .as_i64()
            // Coordinates land on whole pixels in the end, so rounding is what
            // the compositor means rather than a loss.
            .or_else(|| number.as_f64().map(|value| value.round() as i64))
            .ok_or_else(|| serde::de::Error::custom("número fuera de rango")),
        other => Err(serde::de::Error::custom(format!(
            "se esperaba un número, llegó {other}"
        ))),
    }
}

/// Same, for the fields Wayfire may omit entirely.
fn optional_number_as_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Option::<serde_json::Value>::deserialize(deserializer)? {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(number)) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|value| value.round() as i64))
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("número fuera de rango")),
        Some(other) => Err(serde::de::Error::custom(format!(
            "se esperaba un número, llegó {other}"
        ))),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Geometry {
    #[serde(deserialize_with = "number_as_i64")]
    pub x: i64,
    #[serde(deserialize_with = "number_as_i64")]
    pub y: i64,
    #[serde(deserialize_with = "number_as_i64")]
    pub width: i64,
    #[serde(deserialize_with = "number_as_i64")]
    pub height: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Size {
    #[serde(deserialize_with = "number_as_i64")]
    pub width: i64,
    #[serde(deserialize_with = "number_as_i64")]
    pub height: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workspace {
    #[serde(rename = "grid_width")]
    #[serde(deserialize_with = "number_as_i64")]
    pub grid_width: i64,
    #[serde(rename = "grid_height")]
    #[serde(deserialize_with = "number_as_i64")]
    pub grid_height: i64,
    #[serde(deserialize_with = "number_as_i64")]
    pub x: i64,
    #[serde(deserialize_with = "number_as_i64")]
    pub y: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Output {
    pub geometry: Geometry,
    #[serde(deserialize_with = "number_as_i64")]
    pub id: i64,
    pub name: String,
    #[serde(rename = "workarea")]
    pub work_area: Geometry,
    #[serde(rename = "workspace")]
    pub workspace: Workspace,
    #[serde(rename = "wset-index")]
    #[serde(deserialize_with = "number_as_i64")]
    pub wset_index: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct View {
    pub activated: bool,
    #[serde(rename = "app-id")]
    pub app_id: Option<String>,
    #[serde(rename = "base-geometry")]
    pub base_geometry: Option<Geometry>,
    pub bbox: Option<Geometry>,
    pub focusable: Option<bool>,
    pub fullscreen: Option<bool>,
    pub geometry: Option<Geometry>,
    #[serde(deserialize_with = "number_as_i64")]
    pub id: i64,
    #[serde(rename = "last-focus-timestamp")]
    #[serde(default, deserialize_with = "optional_number_as_i64")]
    pub last_focus_timestamp: Option<i64>,
    pub layer: Option<String>,
    pub mapped: Option<bool>,
    #[serde(rename = "max-size")]
    pub max_size: Option<Size>,
    #[serde(rename = "min-size")]
    pub min_size: Option<Size>,
    pub minimized: Option<bool>,
    #[serde(rename = "output-id")]
    #[serde(default, deserialize_with = "optional_number_as_i64")]
    pub output_id: Option<i64>,
    #[serde(rename = "output-name")]
    pub output_name: Option<String>,
    #[serde(default, deserialize_with = "optional_number_as_i64")]
    pub parent: Option<i64>,
    #[serde(default, deserialize_with = "optional_number_as_i64")]
    pub pid: Option<i64>,
    pub role: Option<String>,
    pub sticky: Option<bool>,
    #[serde(rename = "tiled-edges")]
    #[serde(default, deserialize_with = "optional_number_as_i64")]
    pub tiled_edges: Option<i64>,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    #[serde(rename = "wset-index")]
    #[serde(default, deserialize_with = "optional_number_as_i64")]
    pub wset_index: Option<i64>,
}

fn fallback_socket_path() -> Option<PathBuf> {
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR")?;
    let wayland_display = env::var("WAYLAND_DISPLAY").ok()?;
    Some(PathBuf::from(runtime_dir).join(format!("wayfire-{}-.socket", wayland_display)))
}

fn socket_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    for variable in ["WAYFIRE_SOCKET", "WAYFIRE_IPC_SOCKET", "_WAYFIRE_SOCKET"] {
        if let Some(value) = env::var_os(variable) {
            candidates.push(PathBuf::from(value));
        }
    }

    if let Some(runtime_socket) = fallback_socket_path() {
        candidates.push(runtime_socket);
    }

    if let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR") {
        let runtime_dir = PathBuf::from(runtime_dir);
        candidates.push(runtime_dir.join("wayfire.socket"));
        candidates.push(runtime_dir.join("wayfire-ipc.socket"));
        candidates.push(runtime_dir.join("wayfire-ipc.sock"));

        if let Ok(entries) = std::fs::read_dir(&runtime_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                    if file_name.starts_with("wayfire-") && file_name.ends_with(".socket") {
                        candidates.push(path);
                    }
                }
            }
        }
    }

    candidates
}

fn is_usable_socket(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.file_type().is_socket())
        .unwrap_or(false)
}

pub struct WayfireClient {
    writer: Arc<AsyncMutex<OwnedWriteHalf>>,
    pending: Arc<AsyncMutex<VecDeque<Value>>>,
    notify: Arc<Notify>,
    request_lock: Arc<AsyncMutex<()>>,
    event_tx: broadcast::Sender<Value>,
    closed: Arc<AtomicBool>,
}

impl WayfireClient {
    pub async fn connect() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut last_error: Option<Box<dyn Error + Send + Sync>> = None;

        loop {
            for socket_path in socket_candidates().into_iter().filter(|c| is_usable_socket(c)) {
                match UnixStream::connect(&socket_path).await {
                    Ok(stream) => {
                        let (reader, writer) = stream.into_split();

                        let writer = Arc::new(AsyncMutex::new(writer));
                        let pending = Arc::new(AsyncMutex::new(VecDeque::new()));
                        let notify = Arc::new(Notify::new());
                        let request_lock = Arc::new(AsyncMutex::new(()));
                        let (event_tx, _) = broadcast::channel(128);
                        let closed = Arc::new(AtomicBool::new(false));

                        Self::spawn_reader(reader, pending.clone(), notify.clone(), event_tx.clone(), closed.clone());

                        return Ok(Self {
                            writer,
                            pending,
                            notify,
                            request_lock,
                            event_tx,
                            closed,
                        });
                    }
                    Err(error) => {
                        last_error = Some(Box::new(error));
                    }
                }
            }

            if Instant::now() >= deadline {
                break;
            }

            sleep(Duration::from_millis(200)).await;
        }

        if let Some(error) = last_error {
            return Err(error);
        }

        Err("No wayfire socket found".into())
    }

    fn spawn_reader(
        mut reader: OwnedReadHalf,
        pending: Arc<AsyncMutex<VecDeque<Value>>>,
        notify: Arc<Notify>,
        event_tx: broadcast::Sender<Value>,
        closed: Arc<AtomicBool>,
    ) {
        tokio::spawn(async move {
            let result: Result<(), ()> = async {
                loop {
                    let mut header = [0u8; 4];
                    reader.read_exact(&mut header).await.map_err(|_| ())?;

                    let len = u32::from_le_bytes(header) as usize;
                    let mut buffer = vec![0u8; len];
                    reader.read_exact(&mut buffer).await.map_err(|_| ())?;

                    let message: Value = serde_json::from_slice(&buffer).map_err(|_| ())?;

                    if message.get("event").is_some() {
                        let _ = event_tx.send(message);
                    } else {
                        pending.lock().await.push_back(message);
                        notify.notify_waiters();
                    }
                }
            }
            .await;

            if result.is_err() {
                closed.store(true, Ordering::SeqCst);
                notify.notify_waiters();
            }
        });
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Value> {
        self.event_tx.subscribe()
    }

    /// True once the socket has dropped and this client can no longer be used.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    pub async fn send_and_wait(&self, method: &str, data: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        if self.closed.load(Ordering::SeqCst) {
            return Err("Wayfire IPC connection closed".into());
        }

        let _request_guard = self.request_lock.lock().await;

        let payload = json!({
            "method": method,
            "data": data,
        });
        let serialized = serde_json::to_vec(&payload)?;
        let len = u32::try_from(serialized.len())?;

        {
            let mut writer = self.writer.lock().await;
            writer.write_all(&len.to_le_bytes()).await?;
            writer.write_all(&serialized).await?;
            writer.flush().await?;
        }

        loop {
            if self.closed.load(Ordering::SeqCst) {
                return Err("Wayfire IPC connection closed".into());
            }

            if let Some(message) = self.pending.lock().await.pop_front() {
                if message.get("event").is_some() {
                    continue;
                }

                if let Some(error) = message.get("error").and_then(Value::as_str) {
                    return Err(error.to_string().into());
                }

                return Ok(message);
            }

            self.notify.notified().await;
        }
    }

    pub async fn list_views_typed(&self) -> Result<Vec<View>, Box<dyn Error + Send + Sync>> {
        let response = self.send_and_wait("window-rules/list-views", Value::Null).await?;
        Ok(serde_json::from_value(response)?)
    }

    pub async fn list_outputs_typed(&self) -> Result<Vec<Output>, Box<dyn Error + Send + Sync>> {
        let response = self.send_and_wait("window-rules/list-outputs", Value::Null).await?;
        Ok(serde_json::from_value(response)?)
    }

    pub async fn set_focus(&self, view_id: u64) -> Result<Value, Box<dyn Error + Send + Sync>> {
        self.send_and_wait("window-rules/focus-view", json!({ "id": view_id })).await
    }

    pub async fn set_minimized(&self, view_id: u64, state: bool) -> Result<Value, Box<dyn Error + Send + Sync>> {
        self.send_and_wait("wm-actions/set-minimized", json!({ "view_id": view_id, "state": state })).await
    }

    pub async fn configure_view_coords(
        &self,
        view_id: u64,
        x: i64,
        y: i64,
        w: i64,
        h: i64,
        output_id: Option<u64>,
    ) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let mut data = json!({
            "id": view_id,
            "geometry": { "x": x, "y": y, "width": w, "height": h }
        });

        if let Some(output_id) = output_id {
            data["output_id"] = json!(output_id);
        }

        self.send_and_wait("window-rules/configure-view", data).await
    }

    pub async fn set_sticky(&self, view_id: u64, state: bool) -> Result<Value, Box<dyn Error + Send + Sync>> {
        self.send_and_wait("wm-actions/set-sticky", json!({ "view_id": view_id, "state": state })).await
    }

    pub async fn set_always_on_top(&self, view_id: u64, state: bool) -> Result<Value, Box<dyn Error + Send + Sync>> {
        self.send_and_wait("wm-actions/set-always-on-top", json!({ "view_id": view_id, "state": state })).await
    }

    #[allow(dead_code)]
    pub async fn send_to_back(&self, view_id: u64) -> Result<Value, Box<dyn Error + Send + Sync>> {
        self.send_and_wait("wm-actions/send-to-back", json!({ "view_id": view_id })).await
    }

    #[allow(dead_code)]
    pub async fn set_view_property(
        &self,
        view_id: u64,
        property: &str,
        value: Value,
    ) -> Result<Value, Box<dyn Error + Send + Sync>> {
        self.send_and_wait("window-rules/set-view-property", json!({
            "id": view_id,
            "property": property,
            "value": value,
        })).await
    }

    #[allow(dead_code)]
    pub async fn list_methods(&self) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let response = self.send_and_wait("list-methods", Value::Null).await?;
        if let Some(methods) = response.get("methods").and_then(|m| m.as_array()) {
            Ok(methods.iter().filter_map(|m| m.as_str().map(String::from)).collect())
        } else {
            Err("unexpected response format".into())
        }
    }

    #[allow(dead_code)]
    pub async fn get_config_option(&self, option: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
        self.send_and_wait("wayfire/get-config-option", json!({ "option": option })).await
    }

    #[allow(dead_code)]
    pub async fn set_config_options(&self, options: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        self.send_and_wait("wayfire/set-config-options", json!({ "config": options })).await
    }
}

/// The live client, replaceable.
///
/// This was a `OnceLock`, which by definition can never be reset: once the
/// socket dropped, `closed` was set and every call returned "Wayfire IPC
/// connection closed" for the rest of the session. If Wayfire restarted — or
/// the socket blipped — the taskbar stayed dead until the shell itself was
/// restarted. A replaceable slot lets a dead client be dropped and a new
/// connection take its place.
static GLOBAL_WAYFIRE_CLIENT: RwLock<Option<Arc<WayfireClient>>> = RwLock::new(None);
static GLOBAL_WAYFIRE_CLIENT_INIT: OnceLock<AsyncMutex<()>> = OnceLock::new();

/// The cached client, unless it has already closed.
fn live_client() -> Option<Arc<WayfireClient>> {
    let guard = GLOBAL_WAYFIRE_CLIENT.read().ok()?;
    guard
        .as_ref()
        .filter(|client| !client.is_closed())
        .cloned()
}

pub async fn get_wayfire_client() -> Option<Arc<WayfireClient>> {
    if let Some(client) = live_client() {
        return Some(client);
    }

    let init_lock = GLOBAL_WAYFIRE_CLIENT_INIT.get_or_init(|| AsyncMutex::new(()));
    let _guard = init_lock.lock().await;

    // Another task may have reconnected while we waited for the lock.
    if let Some(client) = live_client() {
        return Some(client);
    }

    match WayfireClient::connect().await {
        Ok(client) => {
            let client = Arc::new(client);
            if let Ok(mut slot) = GLOBAL_WAYFIRE_CLIENT.write() {
                *slot = Some(client.clone());
            }
            Some(client)
        }
        Err(_) => {
            // Drop the dead client so the next caller retries instead of
            // handing out a connection that can never work again.
            if let Ok(mut slot) = GLOBAL_WAYFIRE_CLIENT.write() {
                *slot = None;
            }
            None
        }
    }
}

#[cfg(test)]
mod geometry_tests {
    use super::*;

    /// What Wayfire 0.11 actually sends. Refusing it made every window list
    /// fail, which is why the panel showed nothing.
    #[test]
    fn floating_point_geometry_is_accepted() {
        let geometry: Geometry =
            serde_json::from_str(r#"{"x":0.0,"y":38.0,"width":1920.0,"height":1042.0}"#)
                .expect("floats are what the compositor sends");

        assert_eq!(geometry.x, 0);
        assert_eq!(geometry.y, 38);
        assert_eq!(geometry.width, 1920);
        assert_eq!(geometry.height, 1042);
    }

    /// Older versions send integers, and an upgrade must not break the other way.
    #[test]
    fn integer_geometry_still_works() {
        let geometry: Geometry =
            serde_json::from_str(r#"{"x":0,"y":38,"width":1920,"height":1042}"#).expect("ints");
        assert_eq!((geometry.x, geometry.height), (0, 1042));
    }

    /// A fractional coordinate rounds to the pixel it lands on rather than
    /// being refused.
    #[test]
    fn fractional_values_round_to_a_pixel() {
        let geometry: Geometry =
            serde_json::from_str(r#"{"x":10.6,"y":-0.4,"width":100.5,"height":50.49}"#)
                .expect("fractions");
        assert_eq!((geometry.x, geometry.y, geometry.width, geometry.height), (11, 0, 101, 50));
    }

    #[test]
    fn something_that_is_not_a_number_is_still_refused() {
        assert!(
            serde_json::from_str::<Geometry>(r#"{"x":"0","y":0,"width":0,"height":0}"#).is_err(),
            "a string coordinate is a protocol change worth failing on"
        );
    }
}
