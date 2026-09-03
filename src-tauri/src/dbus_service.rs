use crate::commands::{toggle_control_center, toggle_menu, toggle_search, toggle_session_popup};
use crate::constants::DBUS_SERVICE_NAME;
use crate::logger::{log_info, log_error, log_warning, log_debug};
use futures_util::TryStreamExt;
use tauri::{AppHandle, Emitter};
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

        log_debug(&format!("D-Bus: Método llamado: {}", member));
        match member {
            "OpenMenu" => {
                log_info("D-Bus: Abriendo menú");
                let _ = toggle_menu(self.app_handle.clone());
            }
            "OpenControlCenter" => {
                log_info("D-Bus: Abriendo centro de control");
                // Has to run on the main thread. The centre's layer surface
                // lives in a thread-local registry there, so called straight
                // from this D-Bus worker the toggle looks it up in an empty map,
                // concludes the surface was never built, and does nothing —
                // OpenControlCenter was silently dead over D-Bus.
                //
                // OpenMenu below needs no such care: it goes through Tauri's
                // window API, which marshals to the main thread itself.
                let app_handle = self.app_handle.clone();
                if let Err(e) = self.app_handle.run_on_main_thread(move || {
                    let _ = toggle_control_center(app_handle);
                }) {
                    log_error(&format!("D-Bus: no se pudo alternar el centro de control: {}", e));
                }
            }
            "OpenSearch" | "ToggleSearch" => {
                log_info("D-Bus: Alternando búsqueda");
                let app_handle = self.app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = toggle_search(app_handle).await;
                });
            }
            "OpenSessionPopup" | "PowerButtonPressed" => {
                log_info("D-Bus: Abriendo popup de sesión");
                let app_handle = self.app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = toggle_session_popup("shutdown".to_string(), app_handle).await;
                });
            }
            // Pausar y reanudar el fondo en movimiento desde afuera.
            //
            // Lo usa el temporizador de inactividad: un video decodificando
            // detrás de la pantalla de bloqueo, durante horas, es el gasto más
            // grande y el más inútil de todos. El escritorio no puede darse
            // cuenta solo —la superficie de bloqueo es de otro proceso y la
            // suya sigue mapeada—, así que se lo avisa quien sí sabe.
            "PauseWallpaper" | "ResumeWallpaper" => {
                let reproducir = member == "ResumeWallpaper";
                log_info(&format!(
                    "D-Bus: {} el fondo en movimiento",
                    if reproducir { "reanudando" } else { "pausando" }
                ));
                if let Err(e) = self.app_handle.emit("wallpaper-playback", reproducir) {
                    log_error(&format!("D-Bus: no se pudo avisar al fondo: {}", e));
                }
            }
            // Traer al frente la ventana de una aplicación.
            //
            // Existe acá y no en cada componente porque en Wayland sólo el
            // compositor puede hacerlo, y de todo el escritorio el único que le
            // habla es este proceso. Lo usan el daemon de notificaciones —al
            // hacer clic en una— y la configuración cuando se le pide una
            // sección con la ventana ya abierta.
            //
            // No contesta, como el resto de los métodos de este servicio: quien
            // llama tiene que usar `--no-reply` o no esperar. Y no falla si la
            // aplicación no está abierta; eso es un caso normal, no un error.
            "PresentApp" => {
                match msg.body().deserialize::<String>() {
                    Ok(pedido) => {
                        log_info(&format!("D-Bus: trayendo al frente «{}»", pedido));
                        tauri::async_runtime::spawn(async move {
                            if !crate::window_manager::present::present_app(&pedido).await {
                                log_debug(&format!(
                                    "D-Bus: no hay ninguna ventana de «{}» para mostrar",
                                    pedido
                                ));
                            }
                        });
                    }
                    Err(e) => log_warning(&format!(
                        "D-Bus: PresentApp sin un nombre de aplicación válido: {}",
                        e
                    )),
                }
            }
            _ => {
                log::warn!("D-Bus: Unknown method called: {}", member);
                log_warning(&format!("D-Bus: Método desconocido: {}", member));
            }
        }

        Ok(())
    }
}

/// Inicia el servicio D-Bus en un hilo separado
pub async fn start_dbus_service(app_handle: AppHandle) -> ZbusResult<()> {
    log::info!("Starting D-Bus service...");
    log_info("Iniciando servicio D-Bus...");

    let service = DesktopService::new(app_handle);

    // Conectar al bus de sesión
    let connection = Connection::session().await?;

    // Solicitar el nombre del servicio
    connection.request_name(DBUS_SERVICE_NAME).await?;
    
    log::info!("D-Bus service registered as: {}", DBUS_SERVICE_NAME);
    log_info(&format!("Servicio D-Bus registrado como: {}", DBUS_SERVICE_NAME));

    // Procesar mensajes D-Bus usando stream
    let mut stream = zbus::MessageStream::from(&connection);

    while let Some(msg) = stream.try_next().await? {
        // Verificar si es para nuestro servicio
        if msg.header().destination().map(|d| d.as_str()) == Some(DBUS_SERVICE_NAME) {
            // Manejar la llamada al método
            if let Err(e) = service.handle_method_call(&msg).await {
                log::error!("Error handling D-Bus method call: {}", e);
                log_error(&format!("Error al manejar llamada D-Bus: {}", e));
            }
        }
    }

    Ok(())
}
