use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Listener, Manager};
use tokio::sync::RwLock;
use crate::logger::{log_info, log_error};
use super::Applet;

/// Priority levels for applet startup ordering.
/// Critical applets are started first and awaited before proceeding.
/// Normal applets are spawned concurrently without waiting for completion.
/// Deferred applets are started only after the panel has painted (panel-ready event).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppletPriority {
    /// Must be ready before other applets start (e.g., Audio, Brightness)
    Critical,
    /// Started concurrently after Critical applets are ready (e.g., Battery, Music, Tray, Notifications)
    Normal,
    /// Deferred until after panel first paint (e.g., Bluetooth, Network)
    Deferred,
}

pub struct AppletManager {
    applets: RwLock<HashMap<&'static str, (Arc<dyn Applet>, AppletPriority)>>,
}

impl AppletManager {
    pub fn new() -> Self {
        Self {
            applets: RwLock::new(HashMap::new()),
        }
    }

    /// Register an applet with a given startup priority.
    pub async fn register(&self, applet: impl Applet + 'static, priority: AppletPriority) {
        let applet = Arc::new(applet);
        let name = applet.name();
        let mut applets = self.applets.write().await;
        applets.insert(name, (applet, priority));
    }

    /// Start applets in priority phases:
    /// 1. Critical: started and awaited (must be ready before proceeding)
    /// 2. Normal: spawned concurrently without awaiting completion
    /// 3. Deferred: spawned only after receiving the "panel-ready" event from frontend
    ///
    /// Individual applet failures are logged but do not block other applets.
    pub async fn start_phased(self: Arc<Self>, app: AppHandle) {
        let applets = self.applets.read().await;
        log_info(&format!("Iniciando {} applets en fases", applets.len()));

        // Collect applets by priority
        let mut critical: Vec<(&'static str, Arc<dyn Applet>)> = Vec::new();
        let mut normal: Vec<(&'static str, Arc<dyn Applet>)> = Vec::new();
        let mut deferred: Vec<(&'static str, Arc<dyn Applet>)> = Vec::new();

        for (name, (applet, priority)) in applets.iter() {
            let entry = (*name, applet.clone());
            match priority {
                AppletPriority::Critical => critical.push(entry),
                AppletPriority::Normal => normal.push(entry),
                AppletPriority::Deferred => deferred.push(entry),
            }
        }
        drop(applets); // Release read lock

        // Subscribe to panel-ready BEFORE Phase 1, so the receiver is ready
        // before any applet startup runs. The sender (PanelReadyLatch) was
        // registered in setup before create_panel(), so there's no race.
        let mut ready_rx = {
            let latch = app.state::<crate::PanelReadyLatch>();
            latch.0.subscribe()
        };

        // Phase 1: Start Critical applets concurrently and await all of them
        log_info(&format!("Fase 1: Iniciando {} applets críticos", critical.len()));
        let mut critical_handles = Vec::new();
        for (name, applet) in critical {
            let app_handle = app.clone();
            let handle = tokio::spawn(async move {
                log::info!("Starting critical applet: {}", name);
                log_info(&format!("Iniciando applet crítico: {}", name));
                if let Err(e) = applet.start(app_handle).await {
                    log::error!("Critical applet '{}' failed to start: {}", name, e);
                    log_error(&format!("Applet crítico '{}' falló al iniciar: {}", name, e));
                } else {
                    log::info!("Critical applet '{}' started successfully", name);
                    log_info(&format!("Applet crítico '{}' iniciado correctamente", name));
                }
            });
            critical_handles.push(handle);
        }

        // Await all critical applets (individual failures are logged, not propagated)
        for handle in critical_handles {
            let _ = handle.await;
        }
        log_info("Fase 1 completada: applets críticos listos");

        // Phase 2: Spawn Normal applets without awaiting
        log_info(&format!("Fase 2: Iniciando {} applets normales", normal.len()));
        for (name, applet) in normal {
            let app_handle = app.clone();
            tokio::spawn(async move {
                log::info!("Starting normal applet: {}", name);
                log_info(&format!("Iniciando applet normal: {}", name));
                if let Err(e) = applet.start(app_handle).await {
                    log::error!("Normal applet '{}' failed to start: {}", name, e);
                    log_error(&format!("Applet normal '{}' falló al iniciar: {}", name, e));
                } else {
                    log::info!("Normal applet '{}' started successfully", name);
                    log_info(&format!("Applet normal '{}' iniciado correctamente", name));
                }
            });
        }
        log_info("Fase 2 completada: applets normales lanzados");

        // Phase 3: Deferred applets start after panel-ready event
        if !deferred.is_empty() {
            log_info(&format!("Fase 3: {} applets diferidos esperando panel-ready", deferred.len()));
            let app_handle = app.clone();
            let deferred_applets = deferred;

            // Spawn a task that waits for panel-ready then starts deferred applets
            tokio::spawn(async move {
                // If already ready (panel-ready fired before we subscribed),
                // skip the wait entirely.
                if !*ready_rx.borrow_and_update() {
                    log_info("Esperando evento panel-ready para iniciar applets diferidos");
                    let _ = ready_rx.changed().await;
                    log_info("Evento panel-ready recibido, iniciando applets diferidos");
                } else {
                    log_info("panel-ready ya recibido, iniciando applets diferidos inmediatamente");
                }

                for (name, applet) in deferred_applets {
                    let app_clone = app_handle.clone();
                    tokio::spawn(async move {
                        log::info!("Starting deferred applet: {}", name);
                        log_info(&format!("Iniciando applet diferido: {}", name));
                        if let Err(e) = applet.start(app_clone).await {
                            log::error!("Deferred applet '{}' failed to start: {}", name, e);
                            log_error(&format!("Applet diferido '{}' falló al iniciar: {}", name, e));
                        } else {
                            log::info!("Deferred applet '{}' started successfully", name);
                            log_info(&format!("Applet diferido '{}' iniciado correctamente", name));
                        }
                    });
                }
                log_info("Fase 3 completada: applets diferidos lanzados");
            });
        }
    }

    /// Legacy method - starts all applets concurrently without priority ordering.
    /// Kept for backward compatibility.
    pub async fn start_all(&self, app: AppHandle) {
        let applets = self.applets.read().await;
        log_info(&format!("Iniciando {} applets", applets.len()));
        for (name, (applet, _priority)) in applets.iter() {
            let applet = applet.clone();
            let app_handle = app.clone();
            let applet_name = *name;
            
            log::info!("Starting applet: {}", applet_name);
            log_info(&format!("Iniciando applet: {}", applet_name));
            
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
