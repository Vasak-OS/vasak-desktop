use crate::commands::{
    toggle_control_center, toggle_menu, toggle_session_popup, toggle_wallpaper_picker,
};
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
    /// Cada método despacha su acción y dice qué contestar; quién decide si
    /// eso se manda es [`answer`], en un solo lugar. Antes la mayoría no
    /// contestaba nada y quien llamaba sin `--no-reply` —zbus, `gdbus call` o
    /// `busctl call` por omisión— quedaba colgado los veinticinco segundos del
    /// bus aunque la acción se hubiera hecho (vasak-desktop#140).
    pub async fn handle_method_call(
        &self,
        connection: &Connection,
        msg: &Message,
    ) -> ZbusResult<()> {
        let header = msg.header();
        let member = header.member().map(|m| m.as_str()).unwrap_or("Unknown");

        log_debug(&format!("D-Bus: Método llamado: {}", member));
        let reply = self.dispatch(connection, msg, member);
        answer(connection, msg, reply).await
    }

    /// Hace lo que pide el método y devuelve qué contestarle a quien llamó.
    ///
    /// Los que siguen trabajando en otra tarea (la búsqueda, el diálogo de
    /// sesión, traer una aplicación al frente) contestan apenas la despachan:
    /// quien llama espera saber que el pedido llegó, no que terminó.
    fn dispatch(&self, connection: &Connection, msg: &Message, member: &str) -> Reply {
        match member {
            "OpenMenu" => {
                log_info("D-Bus: Abriendo menú");
                // Sin rectángulo: no lo abrió un clic en el panel. Se ancla igual
                // al botón del menú (ver `windows_apps/menu.rs`), no al centro.
                match toggle_menu(self.app_handle.clone(), None) {
                    Ok(()) => Reply::Done,
                    Err(error) => {
                        log_error(&format!("D-Bus: no se pudo alternar el menú: {}", error));
                        Reply::Failed(format!("no se pudo alternar el menú: {}", error))
                    }
                }
            }
            "OpenControlCenter" => {
                log_info("D-Bus: Abriendo centro de control");
                // Has to run on the main thread. The centre's layer surface
                // lives in a thread-local registry there, so called straight
                // from this D-Bus worker the toggle looks it up in an empty map,
                // concludes the surface was never built, and does nothing —
                // OpenControlCenter was silently dead over D-Bus.
                //
                // OpenMenu above needs no such care: the anchored applets
                // marshal to the main thread themselves.
                let app_handle = self.app_handle.clone();
                match self.app_handle.run_on_main_thread(move || {
                    let _ = toggle_control_center(app_handle);
                }) {
                    Ok(()) => Reply::Done,
                    Err(e) => {
                        log_error(&format!(
                            "D-Bus: no se pudo alternar el centro de control: {}",
                            e
                        ));
                        Reply::Failed(format!("no se pudo alternar el centro de control: {}", e))
                    }
                }
            }
            // La búsqueda global ya no vive acá: se fue a `vasak-prism`, que
            // como aplicación aparte puede crecer y quedarse residente, que es
            // lo que la hace instantánea.
            //
            // Esto queda como **reenvío** y no se borra junto con lo demás:
            // cualquier cosa que llame a este nombre —un atajo viejo, un script,
            // una configuración que nadie migró— seguiría llamándolo, y sin
            // reenvío no pasaría nada de nada.
            "OpenSearch" | "ToggleSearch" => {
                log_info("D-Bus: reenviando la búsqueda a vasak-prism");
                let connection = connection.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = forward_to_prism(&connection).await {
                        log_error(&format!(
                            "D-Bus: no se pudo alternar el lanzador: {}",
                            error
                        ));
                    }
                });
                Reply::Done
            }
            "OpenSessionPopup" | "PowerButtonPressed" => {
                log_info("D-Bus: Abriendo popup de sesión");
                let app_handle = self.app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = toggle_session_popup("shutdown".to_string(), app_handle).await;
                });
                Reply::Done
            }
            // Borrar el historial de tiempo de pantalla desde afuera: lo usa el
            // botón de Configuración (el interruptor no hace falta acá, va por
            // `screen_time.enabled` en `vasak.conf` como cualquier ajuste).
            "ClearScreenTime" => {
                log_info("D-Bus: borrando el historial de tiempo de pantalla");
                // El borrado vive ahora en el servicio de salud; el escritorio
                // sólo reenvía el pedido que le llega de Configuración.
                let app_handle = self.app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = crate::screen_time::clear(&app_handle).await {
                        log_error(&format!(
                            "D-Bus: no se pudo borrar el tiempo de pantalla: {error}"
                        ));
                    }
                });
                Reply::Done
            }
            // El selector rápido de fondos (vasak-desktop#133). Para un atajo de
            // teclado de Wayfire, como `OpenMenu`.
            "OpenWallpaperPicker" => {
                log_info("D-Bus: alternando el selector de fondos");
                match toggle_wallpaper_picker(self.app_handle.clone()) {
                    Ok(()) => Reply::Done,
                    Err(error) => {
                        log_error(&format!(
                            "D-Bus: no se pudo alternar el selector de fondos: {}",
                            error
                        ));
                        Reply::Failed(format!(
                            "no se pudo alternar el selector de fondos: {}",
                            error
                        ))
                    }
                }
            }
            // Pausar y reanudar el fondo en movimiento desde afuera.
            //
            // Lo usa el temporizador de inactividad: un video decodificando
            // detrás de la pantalla de bloqueo, durante horas, es el gasto más
            // grande y el más inútil de todos. El escritorio no puede darse
            // cuenta solo —la superficie de bloqueo es de otro proceso y la
            // suya sigue mapeada—, así que se lo avisa quien sí sabe.
            "PauseWallpaper" | "ResumeWallpaper" => {
                let play = member == "ResumeWallpaper";
                log_info(&format!(
                    "D-Bus: {} el fondo en movimiento",
                    if play { "reanudando" } else { "pausando" }
                ));
                match self.app_handle.emit("wallpaper-playback", play) {
                    Ok(()) => Reply::Done,
                    Err(e) => {
                        log_error(&format!("D-Bus: no se pudo avisar al fondo: {}", e));
                        Reply::Failed(format!("no se pudo avisar al fondo: {}", e))
                    }
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
            // No falla si la aplicación no está abierta; eso es un caso normal,
            // no un error.
            "PresentApp" => match msg.body().deserialize::<String>() {
                Ok(requested) => {
                    log_info(&format!("D-Bus: trayendo al frente «{}»", requested));
                    tauri::async_runtime::spawn(async move {
                        if !crate::window_manager::present::present_app(&requested).await {
                            log_debug(&format!(
                                "D-Bus: no hay ninguna ventana de «{}» para mostrar",
                                requested
                            ));
                        }
                    });
                    Reply::Done
                }
                Err(e) => {
                    log_warning(&format!(
                        "D-Bus: PresentApp sin un nombre de aplicación válido: {}",
                        e
                    ));
                    Reply::InvalidArgs("PresentApp espera el nombre de la aplicación")
                }
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
                let windows = self.open_windows();
                log_debug(&format!(
                    "D-Bus: ListWindows devolvió {} ventanas",
                    windows.len()
                ));

                let body = serde_json::to_string(&windows).unwrap_or_else(|error| {
                    log_error(&format!(
                        "D-Bus: no se pudieron serializar las ventanas: {}",
                        error
                    ));
                    "[]".to_string()
                });

                Reply::Text(body)
            }
            // Traerla al frente. **Nunca** minimiza, que es la diferencia con el
            // botón del panel: elegir una ventana en una lista de resultados no
            // puede esconderla. Quien lo llama está esperando la respuesta para
            // esconder su propia ventana.
            "PresentWindow" => match msg.body().deserialize::<String>() {
                Ok(id) => {
                    log_info(&format!("D-Bus: presentando la ventana {}", id));
                    match self.present(&id) {
                        Ok(()) => Reply::Done,
                        Err(error) => {
                            log_warning(&format!("D-Bus: no se pudo presentar {}: {}", id, error));
                            Reply::Failed(error)
                        }
                    }
                }
                Err(e) => {
                    log_warning(&format!(
                        "D-Bus: PresentWindow sin un identificador válido: {}",
                        e
                    ));
                    Reply::InvalidArgs("PresentWindow espera el identificador de la ventana")
                }
            },
            _ => {
                log::warn!("D-Bus: Unknown method called: {}", member);
                log_warning(&format!("D-Bus: Método desconocido: {}", member));
                Reply::UnknownMethod(member.to_string())
            }
        }
    }

    /// Las ventanas que hay, o ninguna si no se pudieron leer.
    ///
    /// Sin lista es un caso normal —el compositor puede estar ocupado— y no un
    /// error que valga la pena devolver: quien preguntó muestra las demás cosas
    /// que encontró y listo.
    fn open_windows(&self) -> Vec<crate::window_manager::WindowInfo> {
        let state = self.app_handle.state::<crate::structs::WMState>();
        let Ok(manager) = state.window_manager.try_read() else {
            log_debug("D-Bus: el gestor de ventanas está ocupado");
            return Vec::new();
        };

        manager.get_window_list().unwrap_or_else(|error| {
            log_error(&format!("D-Bus: no se pudo listar las ventanas: {}", error));
            Vec::new()
        })
    }

    fn present(&self, id: &str) -> Result<(), String> {
        let state = self.app_handle.state::<crate::structs::WMState>();
        let manager = state
            .window_manager
            .try_read()
            .map_err(|_| "el gestor de ventanas está ocupado".to_string())?;

        manager
            .present_window(id)
            .map_err(|error| error.to_string())
    }
}

/// Lo que se le contesta a quien llamó a un método.
#[derive(Debug, PartialEq)]
pub(crate) enum Reply {
    /// Hecho (o despachado): un retorno vacío.
    Done,
    /// Hecho, con un texto de vuelta (`ListWindows`).
    Text(String),
    /// No se pudo: `org.vasak.os.Desktop.Error` con el motivo.
    Failed(String),
    /// Los argumentos no son los que el método espera.
    InvalidArgs(&'static str),
    /// Un nombre que este servicio no tiene.
    UnknownMethod(String),
}

const ERROR_FAILED: &str = "org.vasak.os.Desktop.Error";
const ERROR_INVALID_ARGS: &str = "org.freedesktop.DBus.Error.InvalidArgs";
const ERROR_UNKNOWN_METHOD: &str = "org.freedesktop.DBus.Error.UnknownMethod";

/// Si quien llamó avisó que no espera respuesta (`--no-reply`,
/// `--expect-reply=no`): a ése no se le contesta nada, ni siquiera un error.
fn expects_reply(msg: &Message) -> bool {
    !msg.primary_header()
        .flags()
        .contains(zbus::message::Flags::NoReplyExpected)
}

/// Le manda a quien llamó la respuesta de su método, salvo que haya pedido
/// no recibir ninguna. Es el único lugar que contesta: así ningún método puede
/// olvidarse y dejar a alguien esperando los veinticinco segundos del bus.
pub(crate) async fn answer(connection: &Connection, msg: &Message, reply: Reply) -> ZbusResult<()> {
    if !expects_reply(msg) {
        return Ok(());
    }

    match reply {
        Reply::Done => connection.reply(msg, &()).await,
        Reply::Text(body) => connection.reply(msg, &body).await,
        Reply::Failed(reason) => connection.reply_error(msg, ERROR_FAILED, &reason).await,
        Reply::InvalidArgs(reason) => {
            connection
                .reply_error(msg, ERROR_INVALID_ARGS, &reason.to_string())
                .await
        }
        Reply::UnknownMethod(member) => {
            connection
                .reply_error(
                    msg,
                    ERROR_UNKNOWN_METHOD,
                    &format!("org.vasak.os.Desktop no tiene el método «{}»", member),
                )
                .await
        }
    }
}

/// El nombre que toma el lanzador en el bus de sesión, y dónde vive su objeto.
///
/// Escritos acá y no importados: `vasak-prism` es otro paquete y otro proceso,
/// y lo único que los une es este nombre. Depender de su crate para tres cadenas
/// ataría la compilación de todo el escritorio a la del lanzador.
const PRISM_NAME: &str = "ar.net.vasak.Prism";
const PRISM_PATH: &str = "/ar/net/vasak/Prism";

/// Le pide al lanzador que aparezca o se esconda.
///
/// No hace falta que esté corriendo: el paquete instala su archivo de activación
/// por D-Bus, así que el bus lo levanta con esta misma llamada. Lo que sí puede
/// fallar es que no esté instalado, y eso se anota — no se cae nada.
async fn forward_to_prism(connection: &Connection) -> ZbusResult<()> {
    connection
        .call_method(
            Some(PRISM_NAME),
            PRISM_PATH,
            Some(PRISM_NAME),
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
        // Sólo llamadas a métodos: una señal o un error que llegue con este
        // destino no se contesta.
        if msg.message_type() == zbus::message::Type::MethodCall
            && msg.header().destination().map(|d| d.as_str()) == Some(DBUS_SERVICE_NAME)
        {
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
        assert_eq!(PRISM_PATH, format!("/{}", PRISM_NAME.replace('.', "/")));
    }

    /// Y es el del lanzador, no el de este escritorio.
    ///
    /// Reenviarse a sí mismo sería un bucle: el método vuelve a entrar acá, y
    /// el escritorio se queda hablando solo hasta que expira el tiempo de D-Bus.
    #[test]
    fn el_reenvio_no_apunta_a_este_mismo_servicio() {
        assert_ne!(PRISM_NAME, DBUS_SERVICE_NAME);
    }

    // ── Qué le llega a quien llama (vasak-desktop#140) ────────────────────
    //
    // Un par de conexiones punto a punto sobre un par de sockets: del lado
    // del «servicio» se lee la llamada de verdad, con su encabezado y sus
    // marcas, y se contesta con [`answer`]; del lado de quien llama se mira
    // qué llegó. Sin bus de sesión.
    //
    // Para saber que **no** llegó nada sin esperar a que venza un plazo, el
    // servicio manda una señal testigo después de contestar: en un socket los
    // mensajes llegan en orden, así que si lo primero que ve quien llama es
    // la señal, no hubo respuesta.

    use futures_util::StreamExt;
    // De tokio y no de `std`: zbus se compila con su característica `tokio`
    // (ver `commands/calendar.rs`), y entonces `unix_stream` pide este tipo.
    use tokio::net::UnixStream;
    use zbus::message::{Flags, Type};
    use zbus::{connection, Guid, MessageStream};

    const TEST_PATH: &str = "/org/vasak/os/Desktop";
    const SENTINEL_INTERFACE: &str = "org.vasak.os.Test";
    const SENTINEL_MEMBER: &str = "Sentinel";

    /// Quien llama y el servicio, del otro lado de un par de sockets.
    async fn pair() -> (Connection, Connection) {
        let guid = Guid::generate();
        let (caller, service) = UnixStream::pair().unwrap();
        let service = connection::Builder::unix_stream(service)
            .server(guid)
            .unwrap()
            .p2p()
            .build();
        let caller = connection::Builder::unix_stream(caller).p2p().build();
        futures_util::try_join!(caller, service).unwrap()
    }

    /// La llamada que mandó quien llama y el primer mensaje que recibe después
    /// de que el servicio le contesta `reply`: la respuesta o, si no hubo
    /// ninguna, la señal testigo.
    async fn call_and_answer(member: &str, no_reply: bool, reply: Reply) -> (Message, Message) {
        let (caller, service) = pair().await;
        let mut caller_stream = MessageStream::from(&caller);
        let mut service_stream = MessageStream::from(&service);

        let mut builder = Message::method(TEST_PATH, member)
            .unwrap()
            .interface(DBUS_SERVICE_NAME)
            .unwrap();
        if no_reply {
            builder = builder.with_flags(Flags::NoReplyExpected).unwrap();
        }
        let call = builder.build(&()).unwrap();
        caller.send(&call).await.unwrap();

        let received = loop {
            let msg = service_stream.next().await.unwrap().unwrap();
            if msg.message_type() == Type::MethodCall {
                break msg;
            }
        };
        answer(&service, &received, reply).await.unwrap();
        service
            .emit_signal(
                None::<&str>,
                TEST_PATH,
                SENTINEL_INTERFACE,
                SENTINEL_MEMBER,
                &(),
            )
            .await
            .unwrap();

        let first = caller_stream.next().await.unwrap().unwrap();
        (call, first)
    }

    fn is_sentinel(msg: &Message) -> bool {
        msg.message_type() == Type::Signal
            && msg.header().member().map(|m| m.as_str()) == Some(SENTINEL_MEMBER)
    }

    /// Que `msg` sea la respuesta a `call`, y no otra cosa ni la señal testigo.
    fn assert_answers(call: &Message, msg: &Message, expected: Type) {
        assert!(
            !is_sentinel(msg),
            "quien llamó no recibió ninguna respuesta: se habría quedado esperando el plazo del bus"
        );
        assert_eq!(msg.message_type(), expected);
        assert_eq!(
            msg.header().reply_serial(),
            Some(call.primary_header().serial_num()),
            "la respuesta no corresponde a la llamada"
        );
    }

    fn error_name(msg: &Message) -> Option<String> {
        msg.header().error_name().map(|name| name.to_string())
    }

    #[tokio::test]
    async fn un_metodo_que_se_hizo_contesta_con_un_retorno_vacio() {
        let (call, reply) = call_and_answer("OpenMenu", false, Reply::Done).await;

        assert_answers(&call, &reply, Type::MethodReturn);
        assert!(reply.body().signature().is_none());
    }

    #[tokio::test]
    async fn un_metodo_con_texto_lo_devuelve_como_cadena() {
        let (call, reply) =
            call_and_answer("ListWindows", false, Reply::Text("[]".to_string())).await;

        assert_answers(&call, &reply, Type::MethodReturn);
        assert_eq!(reply.body().deserialize::<String>().unwrap(), "[]");
    }

    #[tokio::test]
    async fn un_metodo_que_fallo_devuelve_el_error_del_escritorio_con_el_motivo() {
        let (call, reply) = call_and_answer(
            "PresentWindow",
            false,
            Reply::Failed("no hay tal ventana".to_string()),
        )
        .await;

        assert_answers(&call, &reply, Type::Error);
        assert_eq!(
            error_name(&reply).as_deref(),
            Some("org.vasak.os.Desktop.Error")
        );
        assert_eq!(
            reply.body().deserialize::<String>().unwrap(),
            "no hay tal ventana"
        );
    }

    #[tokio::test]
    async fn argumentos_que_no_sirven_devuelven_invalid_args() {
        let (call, reply) = call_and_answer(
            "PresentApp",
            false,
            Reply::InvalidArgs("PresentApp espera el nombre de la aplicación"),
        )
        .await;

        assert_answers(&call, &reply, Type::Error);
        assert_eq!(
            error_name(&reply).as_deref(),
            Some("org.freedesktop.DBus.Error.InvalidArgs")
        );
        assert_eq!(
            reply.body().deserialize::<String>().unwrap(),
            "PresentApp espera el nombre de la aplicación"
        );
    }

    #[tokio::test]
    async fn un_metodo_desconocido_devuelve_unknown_method_con_su_nombre() {
        let (call, reply) = call_and_answer(
            "NoExiste",
            false,
            Reply::UnknownMethod("NoExiste".to_string()),
        )
        .await;

        assert_answers(&call, &reply, Type::Error);
        assert_eq!(
            error_name(&reply).as_deref(),
            Some("org.freedesktop.DBus.Error.UnknownMethod")
        );
        let text = reply.body().deserialize::<String>().unwrap();
        assert!(
            text.contains("NoExiste"),
            "el error no dice qué método faltó: {text}"
        );
    }

    /// Con `--no-reply` no se contesta nada, ni lo que salió bien ni un error:
    /// del otro lado nadie lo espera.
    #[tokio::test]
    async fn con_no_reply_expected_no_se_contesta_ninguna_variante() {
        let replies = [
            ("OpenMenu", Reply::Done),
            ("ListWindows", Reply::Text("[]".to_string())),
            (
                "PresentWindow",
                Reply::Failed("no hay tal ventana".to_string()),
            ),
            ("PresentApp", Reply::InvalidArgs("falta el nombre")),
            ("NoExiste", Reply::UnknownMethod("NoExiste".to_string())),
        ];

        for (member, reply) in replies {
            let label = format!("{reply:?}");
            let (_call, first) = call_and_answer(member, true, reply).await;
            assert!(
                is_sentinel(&first),
                "{member} con NO_REPLY_EXPECTED recibió una respuesta ({label}): {:?} {:?}",
                first.message_type(),
                error_name(&first)
            );
        }
    }
}
