use super::{WindowInfo, WindowManagerBackend};
use serde_json::Value;
use std::env;
use std::io::{self, BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::Sender;
use std::thread;

pub struct WaylandManager {
    instance_signature: String,
    runtime_dir: PathBuf,
}

impl WaylandManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let runtime_dir = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::other("XDG_RUNTIME_DIR is not set"))?;

        let instance_signature = env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .ok()
            .filter(|v| !v.is_empty())
            .or_else(|| Self::discover_instance_signature(&runtime_dir).ok())
            .ok_or_else(|| io::Error::other("No se encontró una instancia de Hyprland activa"))?;

        Ok(Self {
            instance_signature,
            runtime_dir,
        })
    }

    fn discover_instance_signature(runtime_dir: &Path) -> io::Result<String> {
        let hypr_dir = runtime_dir.join("hypr");
        let entries = std::fs::read_dir(&hypr_dir)?;

        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let instance_path = entry.path();
            let has_socket = instance_path.join(".socket.sock").exists()
                || instance_path.join(".socket2.sock").exists();

            if has_socket {
                if let Some(name) = instance_path.file_name().and_then(|n| n.to_str()) {
                    return Ok(name.to_string());
                }
            }
        }

        Err(io::Error::other("No Hyprland socket found in XDG_RUNTIME_DIR/hypr"))
    }

    fn socket2_path(&self) -> PathBuf {
        self.runtime_dir
            .join("hypr")
            .join(&self.instance_signature)
            .join(".socket2.sock")
    }

    fn run_hyprctl(&self, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
        let output = Command::new("hyprctl")
            .env("HYPRLAND_INSTANCE_SIGNATURE", &self.instance_signature)
            .env("XDG_RUNTIME_DIR", &self.runtime_dir)
            .args(args)
            .output()
            .map_err(|e| io::Error::other(format!("Failed to execute hyprctl {:?}: {}", args, e)))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(format!(
                "hyprctl {:?} failed: {}",
                args,
                String::from_utf8_lossy(&output.stderr)
            )
            .into())
        }
    }

    fn normalized_window_selector(&self, win_id: &str) -> String {
        if win_id.starts_with("address:") {
            win_id.to_string()
        } else {
            format!("address:{}", win_id)
        }
    }

    fn parse_clients(&self, payload: &str) -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>> {
        let value: Value = serde_json::from_str(payload)?;
        let clients = value
            .as_array()
            .ok_or_else(|| io::Error::other("Hyprland clients response was not an array"))?;

        let mut windows = Vec::new();

        for client in clients {
            let address = client
                .get("address")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();

            if address.is_empty() {
                continue;
            }

            let title = client
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();

            let class_name = client
                .get("class")
                .or_else(|| client.get("initialClass"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();

            let hidden = client
                .get("hidden")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            let urgent = client
                .get("urgent")
                .and_then(Value::as_bool)
                .or_else(|| client.get("attention").and_then(Value::as_bool));

            windows.push(WindowInfo {
                id: address,
                title,
                is_minimized: hidden,
                icon: class_name,
                demands_attention: urgent,
            });
        }

        windows.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(windows)
    }
}

impl WindowManagerBackend for WaylandManager {
    fn get_window_list(&mut self) -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>> {
        let clients = self.run_hyprctl(&["-j", "clients"])?;
        self.parse_clients(&clients)
    }

    fn setup_event_monitoring(&mut self, tx: Sender<()>) -> Result<(), Box<dyn std::error::Error>> {
        let socket_path = self.socket2_path();

        match UnixStream::connect(&socket_path) {
            Ok(stream) => {
                let _ = tx.send(());

                thread::spawn(move || {
                    let mut reader = BufReader::new(stream);
                    let mut line = String::new();

                    loop {
                        line.clear();

                        match reader.read_line(&mut line) {
                            Ok(0) => break,
                            Ok(_) => {
                                if !line.trim().is_empty() {
                                    let _ = tx.send(());
                                }
                            }
                            Err(e) => {
                                log::warn!("Hyprland event socket closed: {}", e);
                                break;
                            }
                        }
                    }
                });
            }
            Err(e) => {
                log::warn!(
                    "Unable to connect to Hyprland event socket {}: {}",
                    socket_path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    fn toggle_window(&self, win_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let selector = self.normalized_window_selector(win_id);
        self.run_hyprctl(&["dispatch", "focuswindow", &selector])?;
        Ok(())
    }
}

impl Default for WaylandManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize WaylandManager")
    }
}