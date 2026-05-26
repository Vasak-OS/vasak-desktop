use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::env;
use std::error::Error;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use tokio::sync::{broadcast, Mutex as AsyncMutex, Notify};
use tokio::time::{sleep, Duration, Instant};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Geometry {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Size {
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workspace {
    #[serde(rename = "grid_width")]
    pub grid_width: i64,
    #[serde(rename = "grid_height")]
    pub grid_height: i64,
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Output {
    pub geometry: Geometry,
    pub id: i64,
    pub name: String,
    #[serde(rename = "workarea")]
    pub work_area: Geometry,
    #[serde(rename = "workspace")]
    pub workspace: Workspace,
    #[serde(rename = "wset-index")]
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
    pub id: i64,
    #[serde(rename = "last-focus-timestamp")]
    pub last_focus_timestamp: Option<i64>,
    pub layer: Option<String>,
    pub mapped: Option<bool>,
    #[serde(rename = "max-size")]
    pub max_size: Option<Size>,
    #[serde(rename = "min-size")]
    pub min_size: Option<Size>,
    pub minimized: Option<bool>,
    #[serde(rename = "output-id")]
    pub output_id: Option<i64>,
    #[serde(rename = "output-name")]
    pub output_name: Option<String>,
    pub parent: Option<i64>,
    pub pid: Option<i64>,
    pub role: Option<String>,
    pub sticky: Option<bool>,
    #[serde(rename = "tiled-edges")]
    pub tiled_edges: Option<i64>,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    #[serde(rename = "wset-index")]
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
    events: Arc<AsyncMutex<VecDeque<Value>>>,
    notify: Arc<Notify>,
    request_lock: Arc<AsyncMutex<()>>,
    event_tx: broadcast::Sender<Value>,
}

impl WayfireClient {
    pub async fn connect() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut last_error: Option<Box<dyn Error + Send + Sync>> = None;

        loop {
            if let Some(socket_path) = socket_candidates()
                .into_iter()
                .find(|candidate| is_usable_socket(candidate))
            {
                match UnixStream::connect(&socket_path).await {
                    Ok(stream) => {
                        let (reader, writer) = stream.into_split();

                        let writer = Arc::new(AsyncMutex::new(writer));
                        let pending = Arc::new(AsyncMutex::new(VecDeque::new()));
                        let events = Arc::new(AsyncMutex::new(VecDeque::new()));
                        let notify = Arc::new(Notify::new());
                        let request_lock = Arc::new(AsyncMutex::new(()));
                        let (event_tx, _) = broadcast::channel(128);

                        Self::spawn_reader(reader, pending.clone(), events.clone(), notify.clone(), event_tx.clone());

                        return Ok(Self {
                            writer,
                            pending,
                            events,
                            notify,
                            request_lock,
                            event_tx,
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
        events: Arc<AsyncMutex<VecDeque<Value>>>,
        notify: Arc<Notify>,
        event_tx: broadcast::Sender<Value>,
    ) {
        tokio::spawn(async move {
            loop {
                let mut header = [0u8; 4];
                if reader.read_exact(&mut header).await.is_err() {
                    break;
                }

                let len = u32::from_le_bytes(header) as usize;
                let mut buffer = vec![0u8; len];
                if reader.read_exact(&mut buffer).await.is_err() {
                    break;
                }

                let Ok(message) = serde_json::from_slice::<Value>(&buffer) else {
                    break;
                };

                if message.get("event").is_some() {
                    events.lock().await.push_back(message.clone());
                    let _ = event_tx.send(message.clone());
                }

                pending.lock().await.push_back(message);
                notify.notify_waiters();
            }
        });
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Value> {
        self.event_tx.subscribe()
    }

    pub async fn send_and_wait(&self, method: &str, data: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
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
            if let Some(message) = self.pending.lock().await.pop_front() {
                if message.get("event").is_some() {
                    self.events.lock().await.push_back(message.clone());
                    let _ = self.event_tx.send(message);
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
        layer: Option<&str>,
        exclusive_zone: Option<i64>,
        output_id: Option<u64>,
    ) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let mut data = json!({
            "id": view_id,
            "geometry": { "x": x, "y": y, "width": w, "height": h }
        });

        if let Some(layer) = layer {
            data["layer"] = json!(layer);
        }

        if let Some(exclusive_zone) = exclusive_zone {
            data["exclusive_zone"] = json!(exclusive_zone);
        }

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

    pub async fn send_to_back(&self, view_id: u64) -> Result<Value, Box<dyn Error + Send + Sync>> {
        self.send_and_wait("wm-actions/send-to-back", json!({ "view_id": view_id })).await
    }
}

static GLOBAL_WAYFIRE_CLIENT: OnceLock<Arc<WayfireClient>> = OnceLock::new();
static GLOBAL_WAYFIRE_CLIENT_INIT: OnceLock<AsyncMutex<()>> = OnceLock::new();

pub async fn get_wayfire_client() -> Option<Arc<WayfireClient>> {
    if let Some(client) = GLOBAL_WAYFIRE_CLIENT.get() {
        return Some(client.clone());
    }

    let init_lock = GLOBAL_WAYFIRE_CLIENT_INIT.get_or_init(|| AsyncMutex::new(()));
    let _guard = init_lock.lock().await;

    if let Some(client) = GLOBAL_WAYFIRE_CLIENT.get() {
        return Some(client.clone());
    }

    match WayfireClient::connect().await {
        Ok(client) => {
            let arc = Arc::new(client);
            let _ = GLOBAL_WAYFIRE_CLIENT.set(arc.clone());
            GLOBAL_WAYFIRE_CLIENT.get().cloned()
        }
        Err(_) => None,
    }
}
