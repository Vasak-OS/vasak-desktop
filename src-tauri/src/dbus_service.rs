use crate::commands::{toggle_config_app, toggle_control_center, toggle_menu, toggle_search};
use crate::constants::DBUS_SERVICE_NAME;
use crate::windows_apps::create_file_manager_window;
use futures_util::TryStreamExt;
use tauri::AppHandle;
use zbus::{Connection, Message, Result as ZbusResult};

/// Servicio D-Bus simplificado para controlar la aplicación Vasak Desktop
pub struct DesktopService {
    app_handle: AppHandle,
}

impl DesktopService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// Maneja llamadas a métodos D-Bus
    pub async fn handle_method_call(&self, msg: &Message) -> ZbusResult<()> {
        let header = msg.header();
        let member = header.member().map(|m| m.as_str()).unwrap_or("Unknown");

        match member {
            "OpenMenu" => {
                let _ = toggle_menu(self.app_handle.clone());
            }
            "OpenConfigApp" => {
                let _ = toggle_config_app(self.app_handle.clone());
            }
            "OpenControlCenter" => {
                let _ = toggle_control_center(self.app_handle.clone());
            }
            "OpenSearch" | "ToggleSearch" => {
                let app_handle = self.app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = toggle_search(app_handle).await;
                });
            }
            "OpenFileManager" => {
                let app_handle = self.app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = create_file_manager_window(app_handle, None);
                });
            }
            _ => {
                log::warn!("D-Bus: Unknown method called: {}", member);
            }
        }

        Ok(())
    }
}

/// Inicia el servicio D-Bus en un hilo separado
pub async fn start_dbus_service(app_handle: AppHandle) -> ZbusResult<()> {
    log::info!("Starting D-Bus service...");

    let service = DesktopService::new(app_handle);

    // Conectar al bus de sesión
    let connection = Connection::session().await?;

    // Solicitar el nombre del servicio
    connection.request_name(DBUS_SERVICE_NAME).await?;
    
    log::info!("D-Bus service registered as: {}", DBUS_SERVICE_NAME);

    // Procesar mensajes D-Bus usando stream
    let mut stream = zbus::MessageStream::from(&connection);

    while let Some(msg) = stream.try_next().await? {
        // Verificar si es para nuestro servicio
        if msg.header().destination().map(|d| d.as_str()) == Some(DBUS_SERVICE_NAME) {
            // Manejar la llamada al método
            if let Err(e) = service.handle_method_call(&msg).await {
                log::error!("Error handling D-Bus method call: {}", e);
            }
        }
    }

    Ok(())
}
