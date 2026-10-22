//! El vigía del portapapeles: un cliente data-control que escucha cada cambio de
//! selección, lee el texto y lo guarda.
//!
//! Una conexión de Wayland **propia**, en su propio hilo, aparte de la de GTK:
//! así bloquea esperando eventos sin tocar el bucle principal (el mismo patrón
//! que `screen_time/activity.rs`). Usa `ext-data-control-v1`, el protocolo
//! estándar y no privilegiado con el que un cliente puede hacer de gestor de
//! portapapeles; el compositor de VasakOS (Wayfire) lo ofrece.
//!
//! Por cada selección nueva: se lee **primero** la lista de MIME ofrecidos y se
//! decide si descartarla (secretos, o sin texto; ver [`store::should_skip`]); si
//! no, se lee `text/plain` por un pipe y se guarda. Nuestra propia selección
//! —cuando se elige una entrada (`source.rs`) — vuelve por acá y se deduplica
//! sola: ya es la cabeza del historial.

use std::collections::HashMap;
use std::io::Read;
use std::os::fd::{AsFd, FromRawFd, OwnedFd};
use std::time::Duration;

use wayland_client::backend::ObjectId;
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::{self, ExtDataControlDeviceV1},
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::{self, ExtDataControlOfferV1},
};

use crate::clipboard::{globals, store};
use crate::logger::{log_info, log_warning};

/// Arranca el hilo que vigila el portapapeles. Silencioso si no hay compositor o
/// no ofrece data-control: en ese caso no hay historial y el resto sigue igual.
pub fn start() {
    let spawned = std::thread::Builder::new()
        .name("clipboard-watch".into())
        .spawn(supervise);
    if let Err(error) = spawned {
        log_warning(&format!(
            "[portapapeles] no se pudo arrancar el vigía: {error}"
        ));
    }
}

/// Mantiene el vigía en pie: si la conexión se cae (el escritorio reinicia), se
/// reconecta. Si el compositor no ofrece el protocolo, avisa una vez y para.
fn supervise() {
    loop {
        match run_watch() {
            // Salida limpia: el protocolo no está. No se reintenta.
            Ok(()) => return,
            Err(error) => {
                log_warning(&format!(
                    "[portapapeles] vigía caído, reintentando en 2 s: {error}"
                ));
                std::thread::sleep(Duration::from_secs(2));
            }
        }
    }
}

struct WatchState {
    /// MIME que descartan la entrada entera, además del centinela por omisión.
    denylist: Vec<String>,
    manager: Option<ExtDataControlManagerV1>,
    seat: Option<wl_seat::WlSeat>,
    /// Los MIME que va ofreciendo cada oferta, hasta que llega su `selection`.
    offers: HashMap<ObjectId, Vec<String>>,
}

fn run_watch() -> Result<(), String> {
    let connection = Connection::connect_to_env().map_err(|error| error.to_string())?;
    let mut queue = connection.new_event_queue();
    let qh = queue.handle();
    connection.display().get_registry(&qh, ());

    let mut state = WatchState {
        denylist: Vec::new(),
        manager: None,
        seat: None,
        offers: HashMap::new(),
    };
    queue
        .roundtrip(&mut state)
        .map_err(|error| error.to_string())?;

    let (Some(manager), Some(seat)) = (state.manager.clone(), state.seat.clone()) else {
        log_warning("[portapapeles] el compositor no ofrece ext-data-control; sin historial");
        return Ok(());
    };
    // El device se queda vivo mientras dure el bucle: es quien emite `selection`.
    let _device = manager.get_data_device(&seat, &qh, ());
    log_info("[portapapeles] vigía en marcha");

    loop {
        queue
            .blocking_dispatch(&mut state)
            .map_err(|error| error.to_string())?;
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WatchState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        globals::bind_data_control(registry, event, qh, &mut state.manager, &mut state.seat);
    }
}

globals::ignora_eventos!(WatchState, wl_seat::WlSeat);
globals::ignora_eventos!(WatchState, ExtDataControlManagerV1);

impl Dispatch<ExtDataControlDeviceV1, ()> for WatchState {
    fn event(
        state: &mut Self,
        _: &ExtDataControlDeviceV1,
        event: ext_data_control_device_v1::Event,
        _: &(),
        connection: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            // Una oferta nueva: empieza a juntar sus MIME, que llegan enseguida.
            ext_data_control_device_v1::Event::DataOffer { id } => {
                state.offers.insert(id.id(), Vec::new());
            }
            // El dueño de la selección cambió. `id` es la oferta ahora vigente, o
            // nada si se vació.
            ext_data_control_device_v1::Event::Selection { id } => {
                if let Some(offer) = id {
                    let mimes = state.offers.remove(&offer.id()).unwrap_or_default();
                    if !store::should_skip(&mimes, &state.denylist) {
                        if let Some(mime) = store::pick_text_mime(&mimes) {
                            if let Some(text) = read_offer(&offer, mime, connection) {
                                store::remember(text);
                            }
                        }
                    }
                    offer.destroy();
                }
                // Cualquier oferta que quedó sin convertirse en selección es basura:
                // se limpia para que el mapa no crezca.
                state.offers.clear();
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtDataControlOfferV1, ()> for WatchState {
    fn event(
        state: &mut Self,
        offer: &ExtDataControlOfferV1,
        event: ext_data_control_offer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_data_control_offer_v1::Event::Offer { mime_type } = event {
            state.offers.entry(offer.id()).or_default().push(mime_type);
        }
    }
}

/// Lee el contenido de `offer` en `mime` por un pipe, hasta [`store::MAX_BYTES`].
///
/// El lado del compositor atiende el pedido y la app dueña escribe en el pipe; la
/// lectura bloquea este hilo mientras tanto, lo que está bien: es un hilo propio.
/// Devuelve `None` si no es UTF-8 válido —no es texto que mostrar— o si el pipe
/// falla.
fn read_offer(
    offer: &ExtDataControlOfferV1,
    mime: &str,
    connection: &Connection,
) -> Option<String> {
    let mut fds = [0_i32; 2];
    // O_CLOEXEC para no filtrar el pipe a procesos hijos.
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        log_warning("[portapapeles] no se pudo crear el pipe de lectura");
        return None;
    }
    let read_fd = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let write_fd = unsafe { OwnedFd::from_raw_fd(fds[1]) };

    offer.receive(mime.to_string(), write_fd.as_fd());
    let _ = connection.flush();
    // Cerrar nuestra punta de escritura: sin esto la lectura no vería nunca el
    // EOF, porque el pipe seguiría teniendo un escritor abierto (nosotros).
    drop(write_fd);

    let mut buffer = Vec::new();
    let mut reader = std::fs::File::from(read_fd).take(store::MAX_BYTES);
    if let Err(error) = reader.read_to_end(&mut buffer) {
        log_warning(&format!(
            "[portapapeles] no se pudo leer la oferta: {error}"
        ));
        return None;
    }
    String::from_utf8(buffer).ok()
}
