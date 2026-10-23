//! El escritorio como dueño del portapapeles: al elegir una entrada del
//! historial, se ofrece su texto como selección por data-control y se sirve
//! cuando otra aplicación lo pide.
//!
//! Es la única parte no rutinaria —el escritorio pasa a ser dueño de la
//! selección—, así que cada `serve` corre en su **propio hilo con su propia
//! conexión**, separado del vigía (`watcher.rs`): que sean dos clientes
//! distintos para el compositor es lo que evita el bloqueo mutuo de leernos a
//! nosotros mismos (el vigía lee en su hilo, este hilo sirve en el suyo).
//!
//! El hilo vive mientras seamos dueños: cuando otra app copia algo, o cuando se
//! elige otra entrada, el compositor manda `cancelled` y el hilo termina. No hace
//! falta un registro de hilos: el que sobra se entera y se va solo.

use std::io::Write;
use std::sync::Arc;

use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::{self, ExtDataControlDeviceV1},
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::ExtDataControlOfferV1,
    ext_data_control_source_v1::{self, ExtDataControlSourceV1},
};

use crate::clipboard::globals;
use crate::clipboard::store::TEXT_MIMES;
use crate::logger::log_warning;

/// Pone `text` en el portapapeles, haciéndose dueño de la selección. No bloquea:
/// el trabajo vive en un hilo propio. Silencioso si no hay compositor que ofrezca
/// data-control (elegir no puede hacer más que eso).
pub fn serve(text: String) {
    let spawned = std::thread::Builder::new()
        .name("clipboard-source".into())
        .spawn(move || {
            if let Err(error) = run_source(text.into_bytes()) {
                log_warning(&format!(
                    "[portapapeles] no se pudo poner en el portapapeles: {error}"
                ));
            }
        });
    if let Err(error) = spawned {
        log_warning(&format!(
            "[portapapeles] no se pudo arrancar el hilo de selección: {error}"
        ));
    }
}

struct SourceState {
    bytes: Arc<[u8]>,
    manager: Option<ExtDataControlManagerV1>,
    seat: Option<wl_seat::WlSeat>,
    /// Otra app tomó la selección (o se eligió otra entrada): hay que irse.
    cancelled: bool,
}

fn run_source(bytes: Vec<u8>) -> Result<(), String> {
    let connection = Connection::connect_to_env().map_err(|error| error.to_string())?;
    let mut queue = connection.new_event_queue();
    let qh = queue.handle();
    connection.display().get_registry(&qh, ());

    let mut state = SourceState {
        bytes: bytes.into(),
        manager: None,
        seat: None,
        cancelled: false,
    };
    queue
        .roundtrip(&mut state)
        .map_err(|error| error.to_string())?;

    let (Some(manager), Some(seat)) = (state.manager.clone(), state.seat.clone()) else {
        return Err("el compositor no ofrece ext-data-control".into());
    };

    let device = manager.get_data_device(&seat, &qh, ());
    let source = manager.create_data_source(&qh, ());
    for mime in TEXT_MIMES {
        source.offer(mime.to_string());
    }
    device.set_selection(Some(&source));
    connection.flush().map_err(|error| error.to_string())?;

    // Seguir atendiendo pedidos (`send`) hasta que nos cancelen.
    while !state.cancelled {
        queue
            .blocking_dispatch(&mut state)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

impl Dispatch<wl_registry::WlRegistry, ()> for SourceState {
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

globals::ignora_eventos!(SourceState, wl_seat::WlSeat);
globals::ignora_eventos!(SourceState, ExtDataControlManagerV1);
// El manager nos entrega una oferta por cada selección (la nuestra incluida, que
// el compositor nos refleja). No las leemos —de eso se ocupa el vigía, con su
// propia conexión—, pero el tipo tiene que existir para declararla como hijo.
globals::ignora_eventos!(SourceState, ExtDataControlOfferV1);

impl Dispatch<ExtDataControlDeviceV1, ()> for SourceState {
    fn event(
        _: &mut Self,
        _: &ExtDataControlDeviceV1,
        event: ext_data_control_device_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Como dueños no leemos ninguna oferta, pero sí hay que soltarlas al quedar
        // vigentes, o se filtra un objeto de protocolo por cada cambio de selección.
        match event {
            ext_data_control_device_v1::Event::Selection { id: Some(offer) }
            | ext_data_control_device_v1::Event::PrimarySelection { id: Some(offer) } => {
                offer.destroy();
            }
            _ => {}
        }
    }

    // El compositor crea la oferta como hijo del `data_offer`: hay que declararlo o
    // `blocking_dispatch` entra en pánico y el hilo dueño muere, soltando la
    // selección —elegir una entrada no volvería a ponerla en el portapapeles—.
    wayland_client::event_created_child!(SourceState, ExtDataControlDeviceV1, [
        ext_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ExtDataControlOfferV1, ()),
    ]);
}

impl Dispatch<ExtDataControlSourceV1, ()> for SourceState {
    fn event(
        state: &mut Self,
        source: &ExtDataControlSourceV1,
        event: ext_data_control_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            // Alguien quiere el contenido: se escribe en el fd y se cierra. En un
            // hilo aparte para no trabar el despacho si el lector es lento o el
            // texto grande (el buffer del pipe es chico).
            ext_data_control_source_v1::Event::Send { fd, .. } => {
                let bytes = state.bytes.clone();
                let _ = std::thread::Builder::new()
                    .name("clipboard-send".into())
                    .spawn(move || {
                        let mut file = std::fs::File::from(fd);
                        let _ = file.write_all(&bytes);
                        // `file` se cierra al soltarse: el lector ve el EOF.
                    });
            }
            // Otra selección reemplazó a la nuestra: soltamos la fuente y salimos.
            ext_data_control_source_v1::Event::Cancelled => {
                source.destroy();
                state.cancelled = true;
            }
            _ => {}
        }
    }
}
