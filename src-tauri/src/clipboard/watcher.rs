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
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::time::{Duration, Instant};

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

/// Cuánto se espera, como mucho, a que la app dueña escriba su contenido.
///
/// La lectura corre en el hilo del vigía, que mientras tanto no despacha nada: una
/// app que se quede con la punta de escritura abierta sin escribir colgaría el
/// vigía para siempre. Con un plazo total —no por lectura, así un goteo de a un
/// byte tampoco lo estira— se corta y se sigue. Cinco segundos sobran para
/// cualquier pegado real por un pipe local.
const READ_TIMEOUT: Duration = Duration::from_secs(5);

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
            // La selección primaria (el pegado con el botón del medio) no entra al
            // historial; sólo se libera su oferta, que el compositor crea por cada
            // texto resaltado en cualquier app. Sin esto se filtra un objeto de
            // protocolo por cada resaltado.
            ext_data_control_device_v1::Event::PrimarySelection { id } => {
                if let Some(offer) = id {
                    state.offers.remove(&offer.id());
                    offer.destroy();
                }
            }
            _ => {}
        }
    }

    // El compositor crea la oferta como hijo del `data_offer`: hay que declararlo,
    // o `blocking_dispatch` entra en pánico al recibir el evento y el hilo muere.
    wayland_client::event_created_child!(WatchState, ExtDataControlDeviceV1, [
        ext_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ExtDataControlOfferV1, ()),
    ]);
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

    // Un byte más que el tope basta para que `decode_clipboard` distinga lo que
    // justo entra de lo que se pasa (y se descarta, ni recortado).
    let bytes = read_bounded(read_fd, store::MAX_BYTES + 1, READ_TIMEOUT)?;
    store::decode_clipboard(bytes, store::MAX_BYTES)
}

/// Lee de `fd` hasta el EOF, `limit` bytes o que venza `timeout` —lo que llegue
/// primero—, sin quedarse colgado si el otro extremo no escribe ni cierra.
///
/// El plazo es total y no por lectura: se calcula contra un `deadline` fijo, así
/// un goteo lento no lo renueva. Devuelve `None` si vence el plazo o el pipe
/// falla; si no, los bytes leídos (que pueden pasarse de `limit` por lo que entre
/// en la última vuelta, y entonces `decode_clipboard` los descarta).
fn read_bounded(fd: OwnedFd, limit: u64, timeout: Duration) -> Option<Vec<u8>> {
    let deadline = Instant::now() + timeout;
    let mut file = std::fs::File::from(fd);
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            log_warning("[portapapeles] el dueño no escribió a tiempo: se descarta");
            return None;
        }

        let mut poll_fd = libc::pollfd {
            fd: file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let millis = remaining.as_millis().min(i32::MAX as u128) as i32;
        // Seguro: `poll_fd` vive toda la llamada y es un único descriptor.
        let ready = unsafe { libc::poll(&mut poll_fd, 1, millis) };
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            // Una señal cortó el poll: se reintenta sin estirar el plazo.
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return None;
        }
        if ready == 0 {
            log_warning("[portapapeles] el dueño no escribió a tiempo: se descarta");
            return None;
        }

        match file.read(&mut chunk) {
            Ok(0) => return Some(buffer),
            Ok(read) => {
                buffer.extend_from_slice(&chunk[..read]);
                if buffer.len() as u64 >= limit {
                    return Some(buffer);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Un pipe con las dos puntas, como el que arma `read_offer`.
    fn pipe() -> (OwnedFd, OwnedFd) {
        let mut fds = [0_i32; 2];
        assert_eq!(unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) }, 0);
        unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) }
    }

    #[test]
    fn lee_lo_que_se_escribe_hasta_el_eof() {
        let (read_fd, write_fd) = pipe();
        let mut escritor = std::fs::File::from(write_fd);
        escritor.write_all(b"hola mundo").expect("se escribe");
        drop(escritor); // cerrar la escritura: EOF
        let bytes = read_bounded(read_fd, 1024, Duration::from_secs(5)).expect("hay bytes");
        assert_eq!(bytes, b"hola mundo");
    }

    #[test]
    fn corta_por_el_plazo_si_el_dueno_no_escribe_ni_cierra() {
        let (read_fd, write_fd) = pipe();
        // La punta de escritura queda abierta y muda: sin plazo, la lectura colgaría
        // el vigía para siempre.
        let bytes = read_bounded(read_fd, 1024, Duration::from_millis(200));
        assert!(bytes.is_none());
        drop(write_fd);
    }

    #[test]
    fn se_planta_al_llegar_al_tope() {
        let (read_fd, write_fd) = pipe();
        let mut escritor = std::fs::File::from(write_fd);
        escritor.write_all(&[b'a'; 100]).expect("se escribe");
        drop(escritor);
        // Con el tope en 10 devuelve al menos eso; `decode_clipboard` después lo
        // descarta por pasarse.
        let bytes = read_bounded(read_fd, 10, Duration::from_secs(5)).expect("hay bytes");
        assert!(bytes.len() >= 10);
    }
}
