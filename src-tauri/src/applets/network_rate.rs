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
use std::collections::BTreeSet;
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

/// Por dónde sale el tráfico de esta máquina.
///
/// Antes esto era una lista de nombres —`docker`, `virbr`, `br-`— y después una
/// regla sobre `/sys/class/net`: descartar los puentes y lo que estuviera
/// esclavizado a uno. Las dos estaban mal. La lista no se termina nunca
/// (`podman0`, `cni0`, el próximo runtime), y la regla se comía el caso de una
/// máquina cuya placa de red está adentro de un puente —lo normal con máquinas
/// virtuales—: el puente se descartaba por ser puente y la placa por tener amo,
/// así que **no quedaba nada** y el indicador marcaba cero para siempre.
///
/// Lo que se quiere medir tiene un nombre exacto: por dónde sale el tráfico. Eso
/// lo dice la tabla de rutas, y en ese caso nombra al puente, que es la interfaz
/// por la que efectivamente se sale.
const RUTAS_V4: &str = "/proc/net/route";
const RUTAS_V6: &str = "/proc/net/ipv6_route";

/// Las interfaces con ruta por defecto: las de salida a internet.
///
/// Pueden ser varias —IPv4 e IPv6, o dos salidas a la vez—, y no hay ninguna
/// garantía de que sean una sola.
fn interfaces_de_salida(v4: &str, v6: &str) -> BTreeSet<String> {
    let mut salidas = BTreeSet::new();

    // `Iface Destination Gateway Flags RefCnt Use Metric Mask …`; la ruta por
    // defecto es destino y máscara en cero.
    for linea in v4.lines().skip(1) {
        let campos: Vec<&str> = linea.split_whitespace().collect();

        if campos.len() > 7 && campos[1] == "00000000" && campos[7] == "00000000" {
            salidas.insert(campos[0].to_string());
        }
    }

    // `destino prefijo origen … interfaz`, con la interfaz al final y el destino
    // en ceros con prefijo cero.
    for linea in v6.lines() {
        let campos: Vec<&str> = linea.split_whitespace().collect();

        if campos.len() >= 10
            && campos[0].chars().all(|c| c == '0')
            && campos[1] == "00"
        {
            if let Some(interfaz) = campos.last() {
                salidas.insert((*interfaz).to_string());
            }
        }
    }

    salidas
}

/// Cuando no hay ruta por defecto —sin internet— igual hay tráfico que mostrar:
/// una copia por la red local, por ejemplo. Ahí se cuenta todo menos lo que
/// empieza y termina en esta misma máquina, o lo que ya cuenta su amo.
fn es_local(nombre: &str) -> bool {
    nombre == "lo"
        || nombre.starts_with("veth")
        || std::path::Path::new("/sys/class/net")
            .join(nombre)
            .join("master")
            .exists()
}

/// Qué interfaces se suman ahora mismo.
fn interfaces_a_contar() -> Contadas {
    let v4 = std::fs::read_to_string(RUTAS_V4).unwrap_or_default();
    let v6 = std::fs::read_to_string(RUTAS_V6).unwrap_or_default();
    let salidas = interfaces_de_salida(&v4, &v6);

    if salidas.is_empty() {
        Contadas::TodoMenosLoLocal
    } else {
        Contadas::Estas(salidas)
    }
}

/// El criterio con el que se decide si una interfaz suma.
enum Contadas {
    Estas(BTreeSet<String>),
    TodoMenosLoLocal,
}

impl Contadas {
    fn incluye(&self, nombre: &str) -> bool {
        match self {
            Contadas::Estas(salidas) => salidas.contains(nombre),
            Contadas::TodoMenosLoLocal => !es_local(nombre),
        }
    }
}

/// Suma los contadores de todas las interfaces que sí cuentan.
///
/// El formato de cada línea es `nombre: rx_bytes rx_packets ... tx_bytes ...`,
/// con las dos primeras líneas de encabezado y el nombre pegado a los dos
/// puntos o separado por espacios, según el ancho del nombre.
fn sumar(texto: &str) -> Contadores {
    sumar_con(texto, &interfaces_a_contar())
}

/// La suma de verdad, con el criterio como parámetro: en la máquina sale de la
/// tabla de rutas, y en un test se arma a mano.
fn sumar_con(texto: &str, contadas: &Contadas) -> Contadores {
    let mut total = Contadores {
        recibidos: 0,
        enviados: 0,
    };

    for linea in texto.lines().skip(2) {
        let Some((nombre, resto)) = linea.split_once(':') else {
            continue;
        };

        let nombre = nombre.trim();
        if !contadas.incluye(nombre) {
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

    /// Las rutas de una máquina común: salida por wlan0.
    const RUTAS: &str = "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT
wlan0\t00000000\t016413AC\t0003\t0\t0\t600\t00000000\t0\t0\t0
wlan0\t006413AC\t00000000\t0001\t0\t0\t600\tOOFFFFFF\t0\t0\t0
docker0\t000011AC\t00000000\t0001\t0\t0\t0\t0000FFFF\t0\t0\t0
";

    /// El criterio que usa la máquina, armado desde una tabla de rutas de
    /// mentira: los tests prueban la regla de verdad y no una copia.
    fn contando(v4: &str, v6: &str) -> Contadas {
        let salidas = interfaces_de_salida(v4, v6);

        if salidas.is_empty() {
            Contadas::TodoMenosLoLocal
        } else {
            Contadas::Estas(salidas)
        }
    }

    /// Dos líneas de encabezado y tres interfaces, con el formato real del
    /// kernel: nombres alineados a la derecha y el loopback en el medio.
    const MUESTRA: &str = "Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo:  123456     100    0    0    0     0          0         0   123456     100    0    0    0     0       0          0
  eth0: 1000000    2000    0    0    0     0          0         0   500000    1000    0    0    0     0       0          0
 wlan0:  250000     500    0    0    0     0          0         0   125000     250    0    0    0     0       0          0
";

    #[test]
    fn suma_sólo_la_interfaz_por_la_que_se_sale() {
        let total = sumar_con(MUESTRA, &contando(RUTAS, ""));

        // wlan0 tiene la ruta por defecto; eth0 está en la muestra pero no lleva
        // tráfico a ningún lado, y el loopback nunca cuenta.
        assert_eq!(total.recibidos, 250_000);
        assert_eq!(total.enviados, 125_000);
    }

    #[test]
    fn con_la_placa_adentro_de_un_puente_cuenta_el_puente() {
        // El caso que rompía la regla anterior: `br0` se descartaba por ser
        // puente y `eth0` por tener amo, así que no quedaba nada y el indicador
        // marcaba cero para siempre. La ruta por defecto nombra al puente.
        let rutas = "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT
br0\t00000000\t0164A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0
";
        let texto = "a\nb\n   br0: 900 1 0 0 0 0 0 0 400 1 0 0 0 0 0 0\n  eth0: 900 1 0 0 0 0 0 0 400 1 0 0 0 0 0 0\n";

        let total = sumar_con(texto, &contando(rutas, ""));

        assert_eq!(total.recibidos, 900, "una sola vez, por el puente");
        assert_eq!(total.enviados, 400);
    }

    #[test]
    fn una_salida_por_ipv6_también_cuenta() {
        let v6 = "00000000000000000000000000000000 00 00000000000000000000000000000000 00 fe80000000000000024290fffe9d4d02 00000400 00000001 00000000 00000003 wlan0\n";

        let salidas = interfaces_de_salida("", v6);

        assert!(salidas.contains("wlan0"), "{salidas:?}");
    }

    #[test]
    fn sin_ruta_por_defecto_igual_se_mide_la_red_local() {
        // Sin internet sigue habiendo tráfico que mostrar —una copia por la red
        // de casa—, así que se cuenta todo menos lo que no sale de la máquina.
        let contadas = contando("Iface\tDestination\n", "");

        assert!(matches!(contadas, Contadas::TodoMenosLoLocal));
        assert!(contadas.incluye("wlan0"));
        assert!(!contadas.incluye("lo"));
        assert!(!contadas.incluye("veth1234"));
    }

    #[test]
    fn los_puentes_de_contenedores_no_suman() {
        // docker0 tiene rutas, pero no la de por defecto: lo que sale de un
        // contenedor a internet vuelve a contarse en wlan0.
        let texto = "a\nb\n docker0: 999 1 0 0 0 0 0 0 999 1 0 0 0 0 0 0\n wlan0: 100 1 0 0 0 0 0 0 50 1 0 0 0 0 0 0\n";

        let total = sumar_con(texto, &contando(RUTAS, ""));

        assert_eq!(total.recibidos, 100);
        assert_eq!(total.enviados, 50);
    }

    #[test]
    fn los_puentes_de_cualquier_runtime_quedan_afuera() {
        // `podman0` y `cni0` no estaban en la lista de prefijos que había antes:
        // su tráfico se contaba dos veces, una por el puente y otra por la
        // interfaz real.
        let texto = "a\nb\n podman0: 900 1 0 0 0 0 0 0 900 1 0 0 0 0 0 0\n    cni0: 700 1 0 0 0 0 0 0 700 1 0 0 0 0 0 0\n  wlan0: 100 1 0 0 0 0 0 0 50 1 0 0 0 0 0 0\n";

        let total = sumar_con(texto, &contando(RUTAS, ""));

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
        let total = sumar_con(texto, &contando(RUTAS, ""));

        assert_eq!(total.recibidos, 200, "la línea sin tx_bytes se descarta");
        assert_eq!(total.enviados, 100);
    }

    #[test]
    fn un_archivo_vacio_da_cero() {
        assert_eq!(
            sumar_con("", &contando(RUTAS, "")),
            Contadores {
                recibidos: 0,
                enviados: 0
            }
        );
    }
}
