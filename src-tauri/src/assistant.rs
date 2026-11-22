//! El lado del escritorio de Lince, el asistente residente de VasakOS
//! (`vasak-assistant`): la superficie de chat le habla al servicio
//! `ar.net.vasak.os.Assistant` por el bus de **sesión**.
//!
//! El contrato es **pull, sin señales**: la ventana sondea `Status` y llama
//! `ChatTurn`; el servicio no emite nada, así que acá no hay `watch_signals`.
//!
//! **Sobre los tipos espejados.** El contrato vive en el crate
//! `vasak-assistant-protocol`, que deriva `Type` del zvariant de zbus 5; este
//! escritorio está clavado en zbus 4, cuyo derive es incompatible. Así que las
//! estructuras de abajo lo calcan a mano —si el contrato cambia, este archivo
//! tiene que cambiar con él y nada lo va a recordar—, igual que `connect.rs`
//! hace con vasak-connect. Las firmas están fijadas por el test de contrato de
//! `protocol/src/lib.rs` (`Message` `(ss)`, `PendingAction` `(sssssss)`,
//! `ChatReply` `(sa(sssssss))`, `ModelInfo` `(ssbs)`, `ServiceStatus`
//! `(bs(ssbs)u)`, `Capability` `(sbs)`).
//!
//! **La conversación la posee la conexión del bus.** El servicio ata cada
//! conversación al nombre único de la conexión que la abrió (su remitente), no
//! a este proceso. `dbus_pool` cachea **una** conexión de sesión y reparte
//! clones de ella —un clon de una `zbus::Connection` comparte el mismo socket y
//! el mismo nombre único—, así que todas las llamadas de acá salen del mismo
//! remitente y la conversación sobrevive al matado/recarga del proceso WebKit de
//! la superficie. Si `dbus_pool` abriera una conexión por llamada, cada llamada
//! tendría otro remitente y el servicio no reconocería la conversación.
//!
//! **Puerta de apagado.** `assistant_available` lee el estado del archivo de la
//! unidad de usuario del asistente (`GetUnitFileState`): encendido es
//! `installed && enabled && !masked`, el mismo criterio que `assistant_unit_state`
//! de vasak-settings. Apagar, desde el panel de Configuración, enmascara la
//! unidad, así que una unidad enmascarada (o inexistente) cuenta como apagada y
//! la superficie no se abre. No se activa el servicio por D-Bus.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use zbus::zvariant::Type;

use crate::dbus_pool::DbusPool;
use crate::logger;

const SERVICE: &str = "ar.net.vasak.os.Assistant";
const PATH: &str = "/ar/net/vasak/os/Assistant";

/// El gestor de systemd del usuario, en el bus de sesión. Lo mismo que usa
/// `night_light.rs` para `wlsunset`; acá sólo se lee el estado del archivo de
/// la unidad del asistente.
const SYSTEMD_SERVICE: &str = "org.freedesktop.systemd1";
const SYSTEMD_PATH: &str = "/org/freedesktop/systemd1";
const SYSTEMD_MANAGER_IFACE: &str = "org.freedesktop.systemd1.Manager";
/// La unidad que provee el servicio del asistente, activada por D-Bus.
const ASSISTANT_UNIT: &str = "vasak-assistant.service";

// ── Tipos del cable (calcados del contrato, zbus 4) ─────────────────────────────

/// Un mensaje de una conversación. El rol viaja como texto
/// (`system`/`user`/`assistant`), como lo manda el servicio. Firma `(ss)`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Una acción que el asistente propone y que la persona confirma antes de que
/// ocurra. El borrador completo no viaja: queda en el servicio atado a `id`, y
/// la persona confirma mirando `to`/`subject`/`body_preview`. Firma
/// `(sssssss)`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PendingAction {
    pub id: String,
    pub conversation_id: String,
    pub kind: String,
    pub summary: String,
    pub to: String,
    pub subject: String,
    pub body_preview: String,
}

/// La respuesta de un turno de `ChatTurn`: el texto del asistente y, si
/// corresponde, cero o una acción pendiente de confirmación. Firma
/// `(sa(sssssss))`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChatReply {
    pub text: String,
    pub pending: Vec<PendingAction>,
}

/// El modelo con el que responde el asistente. `local` sale de la dirección
/// efectiva del backend: `false` quiere decir que cada mensaje viaja a `host`
/// —el dato del indicador «esto sale de la caja»—. Firma `(ssbs)`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ModelInfo {
    pub provider: String,
    pub name: String,
    pub local: bool,
    pub host: String,
}

/// Cómo está el servicio. Firma `(bs(ssbs)u)`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ServiceStatus {
    pub backend_available: bool,
    pub reason: String,
    pub model: ModelInfo,
    pub conversations: u32,
}

/// Una capacidad del asistente sobre un dominio. Firma `(sbs)`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Capability {
    pub domain: String,
    pub available: bool,
    pub reason: String,
}

// ── Cliente ─────────────────────────────────────────────────────────────────────

/// Llama un método del asistente por la conexión de sesión compartida y
/// deserializa la respuesta. El servicio, la ruta y el manejo de «no hay bus»
/// los pone `dbus_pool::session_call`; acá se fija sólo el método. El error del
/// servicio (p. ej. `NotFound`, `Busy`, `Unavailable`) vuelve como texto para
/// que la ventana lo muestre.
async fn call<A, R>(app: &AppHandle, method: &str, args: &A) -> Result<R, String>
where
    A: serde::ser::Serialize + Type,
    R: for<'d> Deserialize<'d> + Type,
{
    crate::dbus_pool::session_call(app, SERVICE, PATH, method, args).await
}

/// Cómo está el servicio: si hay modelo, cuál, y si sale de la caja.
#[tauri::command]
pub async fn assistant_status(app: AppHandle) -> Result<ServiceStatus, String> {
    call(&app, "Status", &()).await
}

/// Las capacidades sobre cada dominio. En esta fase las de datos del usuario
/// vienen no disponibles con su motivo, nunca como error.
#[tauri::command]
pub async fn assistant_capabilities(app: AppHandle) -> Result<Vec<Capability>, String> {
    call::<(), Vec<Capability>>(&app, "Capabilities", &()).await
}

/// Abre una conversación nueva, de esta conexión, y devuelve su id.
#[tauri::command]
pub async fn assistant_new_conversation(app: AppHandle) -> Result<String, String> {
    call(&app, "NewConversation", &()).await
}

/// Los mensajes de una conversación, del más viejo al más nuevo. La usa la
/// ventana al montar (tras una recarga de WebKit) para reconstruir el chat.
#[tauri::command]
pub async fn assistant_history(app: AppHandle, id: String) -> Result<Vec<Message>, String> {
    call(&app, "History", &(id,)).await
}

/// Le manda un mensaje a la conversación y devuelve la respuesta del asistente,
/// con la acción que proponga si la hay. No es streaming y puede tardar (hasta
/// el plazo del modelo local): la ventana no debe cortar antes.
#[tauri::command]
pub async fn assistant_chat_turn(
    app: AppHandle,
    id: String,
    content: String,
) -> Result<ChatReply, String> {
    call(&app, "ChatTurn", &(id, content)).await
}

/// Confirma una acción propuesta por su id y la ejecuta (encola el correo).
/// Devuelve un texto para la persona. Sólo tras el clic de confirmar.
#[tauri::command]
pub async fn assistant_confirm_action(app: AppHandle, action_id: String) -> Result<String, String> {
    call(&app, "ConfirmAction", &(action_id,)).await
}

/// Descarta una acción propuesta por su id sin ejecutarla.
#[tauri::command]
pub async fn assistant_cancel_action(app: AppHandle, action_id: String) -> Result<(), String> {
    call::<(String,), ()>(&app, "CancelAction", &(action_id,)).await
}

// ── Puerta de apagado ───────────────────────────────────────────────────────────

/// Si el estado del archivo de la unidad de systemd cuenta como encendido.
///
/// `GetUnitFileState` devuelve `enabled`/`enabled-runtime` cuando está
/// habilitada, `masked` cuando se apagó desde el panel, y `disabled`/`static`/…
/// en los demás casos; una unidad que no existe da un error que se toma afuera
/// como apagado. Así `starts_with("enabled")` es `installed && enabled && !masked`
/// en una sola llamada —el mismo criterio que `assistant_unit_state` de
/// vasak-settings—.
fn available_from_file_state(state: &str) -> bool {
    state.starts_with("enabled")
}

/// Pregúntale al gestor de systemd del usuario por el estado del archivo de la
/// unidad del asistente. `None` si no hay bus, si el gestor no contesta o si la
/// unidad no está instalada (que para esto es lo mismo que apagada).
async fn unit_file_state(app: &AppHandle) -> Option<String> {
    let conn = app.try_state::<DbusPool>()?.session().await?;
    let reply = conn
        .call_method(
            Some(SYSTEMD_SERVICE),
            SYSTEMD_PATH,
            Some(SYSTEMD_MANAGER_IFACE),
            "GetUnitFileState",
            &(ASSISTANT_UNIT,),
        )
        .await
        .ok()?;
    reply.body().deserialize::<String>().ok()
}

/// Si el asistente está encendido: instalado, habilitado y no enmascarado.
/// La comparten el comando [`assistant_available`] y la puerta de
/// `toggle_assistant`/`OpenAssistant`. Ante cualquier duda (sin bus, sin
/// respuesta) devuelve `false`: apagado se ve apagado, nunca roto.
pub async fn is_available(app: &AppHandle) -> bool {
    match unit_file_state(app).await {
        Some(state) => available_from_file_state(&state),
        None => {
            logger::log_info(
                "vasak-assistant: la unidad no está instalada o el gestor de systemd no contestó; \
                 se trata como apagado",
            );
            false
        }
    }
}

/// `true` si el asistente está encendido. La píldora del panel se muestra sólo
/// con esto, y se re-sondea para aparecer/desaparecer al encender o apagar
/// desde Configuración.
#[tauri::command]
pub async fn assistant_available(app: AppHandle) -> bool {
    is_available(&app).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn firma<T: Type>() -> String {
        T::signature().to_string()
    }

    // La guarda de firmas del cable: si alguien reordena o cambia un campo, el
    // contrato con `vasak-assistant` se rompe sin que el compilador avise (dos
    // `String` intercambiadas tienen la misma firma, pero un `bool` movido no).
    #[test]
    fn las_firmas_del_cable_son_las_del_contrato() {
        assert_eq!(firma::<Message>(), "(ss)");
        assert_eq!(firma::<PendingAction>(), "(sssssss)");
        assert_eq!(firma::<ChatReply>(), "(sa(sssssss))");
        assert_eq!(firma::<ModelInfo>(), "(ssbs)");
        assert_eq!(firma::<ServiceStatus>(), "(bs(ssbs)u)");
        assert_eq!(firma::<Capability>(), "(sbs)");
    }

    #[test]
    fn el_asistente_esta_encendido_si_la_unidad_esta_habilitada() {
        assert!(available_from_file_state("enabled"));
        assert!(available_from_file_state("enabled-runtime"));
    }

    #[test]
    fn el_asistente_esta_apagado_si_la_unidad_no_esta_habilitada() {
        for estado in [
            "masked",
            "masked-runtime",
            "disabled",
            "static",
            "linked",
            "",
            "indirect",
        ] {
            assert!(
                !available_from_file_state(estado),
                "{estado} debía contar como apagado"
            );
        }
    }
}
