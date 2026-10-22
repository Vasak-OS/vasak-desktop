//! El enganche de globals de Wayland que comparten el vigía y la fuente.
//!
//! Las dos conexiones (cada una con su hilo y su cola) ligan lo mismo desde el
//! registro —el manager de data-control y el seat—; lo único que cambia es el
//! estado que los guarda. La lógica vive acá una sola vez; cada lado sólo pasa
//! sus campos.

use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Dispatch, QueueHandle};
use wayland_protocols::ext::data_control::v1::client::ext_data_control_manager_v1::ExtDataControlManagerV1;

/// Liga el manager de data-control y el seat cuando el registro los anuncia.
pub(super) fn bind_data_control<S>(
    registry: &wl_registry::WlRegistry,
    event: wl_registry::Event,
    qh: &QueueHandle<S>,
    manager: &mut Option<ExtDataControlManagerV1>,
    seat: &mut Option<wl_seat::WlSeat>,
) where
    S: Dispatch<ExtDataControlManagerV1, ()> + Dispatch<wl_seat::WlSeat, ()> + 'static,
{
    if let wl_registry::Event::Global {
        name, interface, ..
    } = event
    {
        match interface.as_str() {
            "ext_data_control_manager_v1" if manager.is_none() => {
                *manager = Some(registry.bind(name, 1, qh, ()));
            }
            "wl_seat" if seat.is_none() => {
                *seat = Some(registry.bind(name, 1, qh, ()));
            }
            _ => {}
        }
    }
}

/// El seat y el manager no mandan eventos que nos interesen; esto evita repetir
/// los `impl Dispatch` vacíos, uno por cada estado, en los dos archivos.
macro_rules! ignora_eventos {
    ($state:ty, $iface:ty) => {
        impl wayland_client::Dispatch<$iface, ()> for $state {
            fn event(
                _: &mut Self,
                _: &$iface,
                _: <$iface as wayland_client::Proxy>::Event,
                _: &(),
                _: &wayland_client::Connection,
                _: &wayland_client::QueueHandle<Self>,
            ) {
            }
        }
    };
}
pub(super) use ignora_eventos;
