use async_trait::async_trait;
use tauri::AppHandle;
use std::error::Error;

pub mod manager;

// Modules for specific applets
pub mod audio;
pub mod battery;
pub mod bluetooth;
pub mod brightness;
pub mod music;
pub mod network;
pub mod notifications;
pub mod tray;

#[async_trait]
pub trait Applet: Send + Sync {
    /// Returns the unique name of the applet
    fn name(&self) -> &'static str;

    /// Starts the applet. This method should preferably spawn its own long-running task 
    /// if it needs to monitor events, and return quickly.
    /// Or it can return a Future that runs forever, but the Manager will likely 
    /// spawn it.
    /// 
    /// For now, let's assume this function initializes the applet. 
    /// If it blocks, the Manager must spawn it.
    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>>;
}
