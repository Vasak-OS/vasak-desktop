use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::RwLock;
use crate::logger::{log_info, log_error};
use super::Applet;

pub struct AppletManager {
    applets: RwLock<HashMap<&'static str, Arc<dyn Applet>>>,
}

impl AppletManager {
    pub fn new() -> Self {
        Self {
            applets: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, applet: impl Applet + 'static) {
        let applet = Arc::new(applet);
        let mut applets = self.applets.write().await;
        applets.insert(applet.name(), applet);
    }

    pub async fn start_all(&self, app: AppHandle) {
        let applets = self.applets.read().await;
        log_info(&format!("Iniciando {} applets", applets.len()));
        for (name, applet) in applets.iter() {
            let applet = applet.clone();
            let app_handle = app.clone();
            let applet_name = *name;
            
            log::info!("Starting applet: {}", applet_name);
            log_info(&format!("Iniciando applet: {}", applet_name));
            
            // Spawn each applet start in a separate task to prevent one blocking others
            // capturing errors if they occur
            tokio::spawn(async move {
                if let Err(e) = applet.start(app_handle).await {
                    log::error!("Applet '{}' failed to start: {}", applet_name, e);
                    log_error(&format!("Applet '{}' falló al iniciar: {}", applet_name, e));
                } else {
                    log::info!("Applet '{}' started successfully", applet_name);
                    log_info(&format!("Applet '{}' iniciado correctamente", applet_name));
                }
            });
        }
    }
}
