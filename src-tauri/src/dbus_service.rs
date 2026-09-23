use crate::commands::{toggle_control_center, toggle_menu, toggle_session_popup};
use crate::constants::DBUS_SERVICE_NAME;
use crate::logger::{log_debug, log_error, log_info, log_warning};
use futures_util::TryStreamExt;
use tauri::{AppHandle, Emitter, Manager};
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
    ///
    /// La conexión entra porque hay métodos que **contestan**. Los que hubo acá
    /// hasta ahora no contestaban ninguno —quien llamaba usaba `--no-reply`— y
    /// eso alcanzaba mientras todo fuera «abrí esto». Preguntar qué ventanas hay
    /// es otra cosa: sin respuesta no sirve de nada.
    pub async fn handle_method_call(&self, conexion: &Connection, msg: &Message) -> ZbusResult<()> {
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
                    log_error(&format!(
                        "D-Bus: no se pudo alternar el centro de control: {}",
                        e
                    ));
                }
            }
            // La búsqueda global ya no vive acá: se fue a `vasak-prism`, que
            // como aplicación aparte puede crecer y quedarse residente, que es
            // lo que la hace instantánea.
            //
            // Esto queda como **reenvío** y no se borra junto con lo demás:
            // cualquier cosa que llame a este nombre —un atajo viejo, un script,
            // una configuración que nadie migró— seguiría llamándolo, y sin
            // reenvío no pasaría nada de nada. Un método que no existe en este
            // servicio no falla con estrépito: se anota «método desconocido» en
            // un registro que no mira nadie y el atajo queda muerto.
            "OpenSearch" | "ToggleSearch" => {
                log_info("D-Bus: reenviando la búsqueda a vasak-prism");
                let conexion = conexion.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = reenviar_a_prism(&conexion).await {
                        log_error(&format!(
                            "D-Bus: no se pudo alternar el lanzador: {}",
                            error
                        ));
                    }
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
            "PresentApp" => match msg.body().deserialize::<String>() {
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
            },
            // Las ventanas abiertas, para quien las quiera listar sin hablarle
            // al compositor. De todo el escritorio, este proceso es el único que
            // le habla, y el protocolo que haría falta para enumerarlas
            // —`foreign_toplevel`— está repartido por permiso: dárselo a otro
            // programa es darle también los títulos de todo lo que hay abierto.
            //
            // Va en JSON y no como una estructura de D-Bus porque del otro lado
            // se deserializa con serde igual, y una firma de tipos para cinco
            // campos que todavía pueden cambiar es trabajo que se paga dos veces.
            "ListWindows" => {
                let ventanas = self.ventanas_abiertas();
                log_debug(&format!(
                    "D-Bus: ListWindows devolvió {} ventanas",
                    ventanas.len()
                ));

                let cuerpo = serde_json::to_string(&ventanas).unwrap_or_else(|error| {
                    log_error(&format!(
                        "D-Bus: no se pudieron serializar las ventanas: {}",
                        error
                    ));
                    "[]".to_string()
                });

                conexion.reply(msg, &cuerpo).await?;
            }
            // Traerla al frente. **Nunca** minimiza, que es la diferencia con el
            // botón del panel: elegir una ventana en una lista de resultados no
            // puede esconderla.
            //
            // Éste **contesta**, a diferencia de los de más arriba. No es un
            // gusto: quien lo llama está esperando para esconder su propia
            // ventana, y un método que no contesta lo deja colgado hasta que
            // expira el tiempo de D-Bus — veinticinco segundos con el lanzador
            // en pantalla después de haber elegido algo.
            "PresentWindow" => match msg.body().deserialize::<String>() {
                Ok(id) => {
                    log_info(&format!("D-Bus: presentando la ventana {}", id));
                    match self.presentar(&id) {
                        Ok(()) => conexion.reply(msg, &()).await?,
                        Err(error) => {
                            log_warning(&format!("D-Bus: no se pudo presentar {}: {}", id, error));
                            conexion
                                .reply_error(msg, "org.vasak.os.Desktop.Error", &error)
                                .await?
                        }
                    };
                }
                Err(e) => {
                    log_warning(&format!(
                        "D-Bus: PresentWindow sin un identificador válido: {}",
                        e
                    ));
                    conexion
                        .reply_error(
                            msg,
                            "org.freedesktop.DBus.Error.InvalidArgs",
                            &"PresentWindow espera el identificador de la ventana".to_string(),
                        )
                        .await?;
                }
            },
            _ => {
                log::warn!("D-Bus: Unknown method called: {}", member);
                log_warning(&format!("D-Bus: Método desconocido: {}", member));
            }
        }

        Ok(())
    }

    /// Las ventanas que hay, o ninguna si no se pudieron leer.
    ///
    /// Sin lista es un caso normal —el compositor puede estar ocupado— y no un
    /// error que valga la pena devolver: quien preguntó muestra las demás cosas
    /// que encontró y listo.
    fn ventanas_abiertas(&self) -> Vec<crate::window_manager::WindowInfo> {
        let estado = self.app_handle.state::<crate::structs::WMState>();
        let Ok(gestor) = estado.window_manager.try_read() else {
            log_debug("D-Bus: el gestor de ventanas está ocupado");
            return Vec::new();
        };

        gestor.get_window_list().unwrap_or_else(|error| {
            log_error(&format!("D-Bus: no se pudo listar las ventanas: {}", error));
            Vec::new()
        })
    }

    fn presentar(&self, id: &str) -> Result<(), String> {
        let estado = self.app_handle.state::<crate::structs::WMState>();
        let gestor = estado
            .window_manager
            .try_read()
            .map_err(|_| "el gestor de ventanas está ocupado".to_string())?;

        gestor.present_window(id).map_err(|error| error.to_string())
    }
}

/// El nombre que toma el lanzador en el bus de sesión, y dónde vive su objeto.
///
/// Escritos acá y no importados: `vasak-prism` es otro paquete y otro proceso,
/// y lo único que los une es este nombre. Depender de su crate para tres cadenas
/// ataría la compilación de todo el escritorio a la del lanzador.
const PRISM_NOMBRE: &str = "ar.net.vasak.Prism";
const PRISM_RUTA: &str = "/ar/net/vasak/Prism";

/// Le pide al lanzador que aparezca o se esconda.
///
/// No hace falta que esté corriendo: el paquete instala su archivo de activación
/// por D-Bus, así que el bus lo levanta con esta misma llamada. Lo que sí puede
/// fallar es que no esté instalado, y eso se anota — no se cae nada.
async fn reenviar_a_prism(conexion: &Connection) -> ZbusResult<()> {
    conexion
        .call_method(
            Some(PRISM_NOMBRE),
            PRISM_RUTA,
            Some(PRISM_NOMBRE),
            "Toggle",
            &(),
        )
        .await?;

    Ok(())
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
    log_info(&format!(
        "Servicio D-Bus registrado como: {}",
        DBUS_SERVICE_NAME
    ));

    // Procesar mensajes D-Bus usando stream
    let mut stream = zbus::MessageStream::from(&connection);

    while let Some(msg) = stream.try_next().await? {
        // Verificar si es para nuestro servicio
        if msg.header().destination().map(|d| d.as_str()) == Some(DBUS_SERVICE_NAME) {
            // Manejar la llamada al método
            if let Err(e) = service.handle_method_call(&connection, &msg).await {
                log::error!("Error handling D-Bus method call: {}", e);
                log_error(&format!("Error al manejar llamada D-Bus: {}", e));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// El nombre del lanzador en el bus, escrito de tres formas que tienen que
    /// decir lo mismo.
    ///
    /// Son tres cadenas sueltas de otro paquete: nada las comprueba al
    /// compilar, y un nombre mal escrito no falla acá — falla en la máquina de
    /// alguien, con el atajo viejo sin abrir nada y un renglón en el diario que
    /// no mira nadie. Esto ata al menos que las tres sean coherentes entre sí,
    /// que es el error que de verdad pasa: cambiar una y olvidarse de la otra.
    #[test]
    fn el_nombre_y_la_ruta_del_lanzador_se_corresponden() {
        assert_eq!(PRISM_RUTA, format!("/{}", PRISM_NOMBRE.replace('.', "/")));
    }

    /// Y es el del lanzador, no el de este escritorio.
    ///
    /// Reenviarse a sí mismo sería un bucle: el método vuelve a entrar acá, y
    /// el escritorio se queda hablando solo hasta que expira el tiempo de D-Bus.
    #[test]
    fn el_reenvio_no_apunta_a_este_mismo_servicio() {
        assert_ne!(PRISM_NOMBRE, DBUS_SERVICE_NAME);
    }
}
