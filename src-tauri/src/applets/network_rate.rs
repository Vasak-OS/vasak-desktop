//! Cuánto entra y cuánto sale por segundo.
//!
//! Sale de `/proc/net/dev`, que es un archivo de texto con los contadores
//! acumulados del kernel: la tasa es la diferencia entre dos lecturas dividida
//! por el tiempo que pasó. No hay D-Bus de por medio ni un servicio al que
//! preguntarle —NetworkManager informa el estado del enlace, no el caudal—.
//!
//! Se descarta `lo` y las interfaces virtuales de contenedores: lo que se
//! quiere ver en el panel es el tráfico que sale de la máquina, y una copia
//! entre dos procesos locales aparecería como cientos de megas por segundo.

use super::Applet;
use async_trait::async_trait;
use serde::Serialize;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Cada cuánto se mide. Dos segundos alcanzan para que el número se sienta
/// vivo y son ocho lecturas de un archivo en RAM por minuto.
const INTERVALO: Duration = Duration::from_secs(2);

const PROC_NET_DEV: &str = "/proc/net/dev";

pub struct NetworkRateApplet;

#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
pub struct NetworkRate {
    /// Bytes por segundo que bajan.
    pub down: u64,
    /// Bytes por segundo que suben.
    pub up: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Contadores {
    recibidos: u64,
    enviados: u64,
}

/// Las que no cuentan como «internet»: el loopback y los puentes que arman
/// Docker, libvirt, las VPN de tipo bridge y los pares de veth.
fn es_interna(nombre: &str) -> bool {
    nombre == "lo"
        || nombre.starts_with("veth")
        || nombre.starts_with("docker")
        || nombre.starts_with("br-")
        || nombre.starts_with("virbr")
        || nombre.starts_with("vnet")
}

/// Suma los contadores de todas las interfaces que sí cuentan.
///
/// El formato de cada línea es `nombre: rx_bytes rx_packets ... tx_bytes ...`,
/// con las dos primeras líneas de encabezado y el nombre pegado a los dos
/// puntos o separado por espacios, según el ancho del nombre.
fn sumar(texto: &str) -> Contadores {
    let mut total = Contadores {
        recibidos: 0,
        enviados: 0,
    };

    for linea in texto.lines().skip(2) {
        let Some((nombre, resto)) = linea.split_once(':') else {
            continue;
        };

        let nombre = nombre.trim();
        if es_interna(nombre) {
            continue;
        }

        let campos: Vec<&str> = resto.split_whitespace().collect();

        // rx_bytes es el primero de los ocho de recepción; tx_bytes, el primero
        // de los de transmisión.
        let (Some(rx), Some(tx)) = (campos.first(), campos.get(8)) else {
            continue;
        };

        total.recibidos += rx.parse::<u64>().unwrap_or(0);
        total.enviados += tx.parse::<u64>().unwrap_or(0);
    }

    total
}

/// La tasa entre dos lecturas.
///
/// `saturating_sub` no es cosmética: una interfaz que se desconecta se lleva
/// sus contadores, así que el total puede bajar. Sin esto, el resta daría un
/// número enorme por desbordamiento y el panel mostraría un pico que nunca
/// pasó.
fn tasa(antes: Contadores, ahora: Contadores, transcurrido: Duration) -> NetworkRate {
    let segundos = transcurrido.as_secs_f64();

    if segundos <= 0.0 {
        return NetworkRate { down: 0, up: 0 };
    }

    NetworkRate {
        down: (ahora.recibidos.saturating_sub(antes.recibidos) as f64 / segundos) as u64,
        up: (ahora.enviados.saturating_sub(antes.enviados) as f64 / segundos) as u64,
    }
}

#[async_trait]
impl Applet for NetworkRateApplet {
    fn name(&self) -> &'static str {
        "network_rate"
    }

    async fn start(&self, app_handle: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        let mut anterior = match std::fs::read_to_string(PROC_NET_DEV) {
            Ok(texto) => (sumar(&texto), Instant::now()),
            Err(error) => {
                // Sin /proc/net/dev no hay nada que medir; el widget del panel
                // simplemente no aparece.
                log::warn!("[red] No se pudo leer {PROC_NET_DEV}: {error}");
                return Ok(());
            }
        };

        loop {
            tokio::time::sleep(INTERVALO).await;

            let Ok(texto) = std::fs::read_to_string(PROC_NET_DEV) else {
                continue;
            };

            let ahora = (sumar(&texto), Instant::now());
            let medida = tasa(anterior.0, ahora.0, ahora.1.duration_since(anterior.1));
            anterior = ahora;

            let _ = app_handle.emit("network-rate", medida);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dos líneas de encabezado y tres interfaces, con el formato real del
    /// kernel: nombres alineados a la derecha y el loopback en el medio.
    const MUESTRA: &str = "Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo:  123456     100    0    0    0     0          0         0   123456     100    0    0    0     0       0          0
  eth0: 1000000    2000    0    0    0     0          0         0   500000    1000    0    0    0     0       0          0
 wlan0:  250000     500    0    0    0     0          0         0   125000     250    0    0    0     0       0          0
";

    #[test]
    fn suma_las_interfaces_reales_y_saltea_el_loopback() {
        let total = sumar(MUESTRA);

        assert_eq!(total.recibidos, 1_250_000, "eth0 + wlan0, sin lo");
        assert_eq!(total.enviados, 625_000);
    }

    #[test]
    fn ignora_los_puentes_de_contenedores() {
        let texto = format!(
            "a\nb\n docker0: 999 1 0 0 0 0 0 0 999 1 0 0 0 0 0 0\n{}",
            "  eth0: 100 1 0 0 0 0 0 0 50 1 0 0 0 0 0 0\n"
        );

        let total = sumar(&texto);
        assert_eq!(total.recibidos, 100);
        assert_eq!(total.enviados, 50);
    }

    #[test]
    fn la_tasa_es_la_diferencia_por_segundo() {
        let antes = Contadores {
            recibidos: 1_000,
            enviados: 500,
        };
        let ahora = Contadores {
            recibidos: 3_000,
            enviados: 1_500,
        };

        let medida = tasa(antes, ahora, Duration::from_secs(2));

        assert_eq!(medida.down, 1_000);
        assert_eq!(medida.up, 500);
    }

    #[test]
    fn una_interfaz_que_desaparece_no_inventa_un_pico() {
        let antes = Contadores {
            recibidos: 5_000_000,
            enviados: 5_000_000,
        };
        let ahora = Contadores {
            recibidos: 1_000,
            enviados: 1_000,
        };

        let medida = tasa(antes, ahora, Duration::from_secs(2));

        assert_eq!(medida, NetworkRate { down: 0, up: 0 });
    }

    #[test]
    fn sin_tiempo_transcurrido_no_se_divide_por_cero() {
        let c = Contadores {
            recibidos: 10,
            enviados: 10,
        };

        assert_eq!(
            tasa(c, c, Duration::ZERO),
            NetworkRate { down: 0, up: 0 }
        );
    }

    #[test]
    fn una_linea_recortada_no_rompe_la_lectura() {
        let texto = "a\nb\n  eth0: 100\n wlan0: 200 1 0 0 0 0 0 0 100 1 0 0 0 0 0 0\n";
        let total = sumar(texto);

        assert_eq!(total.recibidos, 200, "la línea sin tx_bytes se descarta");
        assert_eq!(total.enviados, 100);
    }

    #[test]
    fn un_archivo_vacio_da_cero() {
        assert_eq!(
            sumar(""),
            Contadores {
                recibidos: 0,
                enviados: 0
            }
        );
    }
}
