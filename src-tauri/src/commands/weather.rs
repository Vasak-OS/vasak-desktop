//! El clima se pide una vez y lo ven todas las ventanas.
//!
//! El panel, el escritorio y el menú son webviews distintas: cada una con su
//! propio JavaScript y su propio `fetch`. Un cache dentro del navegador se
//! duplicaría por ventana —tres pedidos al servicio en cada vuelta— y ninguna
//! se enteraría de lo que ya trajo la otra. El cache vive acá, que es lo único
//! que las tres comparten.
//!
//! Lo que Rust **no** hace es el pedido HTTP: eso sigue en el webview, que ya
//! trae TLS. Traerlo a Rust obligaría a compilar rustls y su cadena entera de
//! dependencias para dos llamados a una API pública, y a mantener un cliente
//! HTTP más en el escritorio.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

/// Cuánto vale un pronóstico antes de volver a pedirlo. Open-Meteo publica por
/// hora; diez minutos es al mismo tiempo fresco y una fracción de lo que el
/// servicio actualiza.
const VIGENCIA: Duration = Duration::from_secs(10 * 60);

/// Cuánto se le espera a la ventana que se comprometió a traerlo. Si se cerró
/// en medio del pedido, o se quedó sin red, otra puede reintentar en vez de
/// dejar el clima congelado hasta que alguien recargue.
const ESPERA_DEL_RECLAMO: Duration = Duration::from_secs(30);

#[derive(Default)]
struct Estado {
    datos: Option<Value>,
    traido: Option<Instant>,
    /// Cuándo alguien se comprometió a traerlo. Sin esto, tres ventanas que
    /// arrancan juntas hacen tres pedidos idénticos al mismo segundo.
    reclamado: Option<Instant>,
    /// Las coordenadas se deducen de la zona horaria y no cambian mientras la
    /// sesión viva: geocodificar una vez alcanza.
    lugar: Option<Value>,
}

impl Estado {
    fn vencido(&self, ahora: Instant) -> bool {
        match self.traido {
            None => true,
            Some(cuando) => ahora.duration_since(cuando) >= VIGENCIA,
        }
    }

    fn reclamo_en_curso(&self, ahora: Instant) -> bool {
        match self.reclamado {
            None => false,
            Some(cuando) => ahora.duration_since(cuando) < ESPERA_DEL_RECLAMO,
        }
    }

    /// Le contesta a una sola ventana que sí. Es la única función que decide
    /// si se toca la red, y por eso es la que se prueba.
    fn reclamar(&mut self, ahora: Instant) -> bool {
        if !self.vencido(ahora) || self.reclamo_en_curso(ahora) {
            return false;
        }

        self.reclamado = Some(ahora);
        true
    }

    fn guardar(&mut self, datos: Value, lugar: Option<Value>, ahora: Instant) {
        self.datos = Some(datos);
        self.traido = Some(ahora);
        self.reclamado = None;
        if lugar.is_some() {
            self.lugar = lugar;
        }
    }

    fn foto(&self, ahora: Instant) -> Option<Pronostico> {
        let datos = self.datos.clone()?;
        let edad = self
            .traido
            .map(|cuando| ahora.duration_since(cuando).as_secs())
            .unwrap_or(0);

        Some(Pronostico {
            datos,
            edad_segundos: edad,
            vencido: self.vencido(ahora),
        })
    }
}

/// Lo que ve el frontend: el pronóstico tal cual lo devolvió el servicio, más
/// la edad, para que pueda mostrar algo viejo mientras pide lo nuevo en vez de
/// dejar el widget vacío.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pronostico {
    pub datos: Value,
    pub edad_segundos: u64,
    pub vencido: bool,
}

#[derive(Default)]
pub struct WeatherCache(Mutex<Estado>);

fn abrir<'a>(cache: &'a State<'_, WeatherCache>) -> std::sync::MutexGuard<'a, Estado> {
    // Un candado envenenado dejaría el clima muerto para toda la sesión por un
    // pánico en otra ventana; el estado que protege se puede reconstruir con un
    // pedido más.
    cache.0.lock().unwrap_or_else(|roto| roto.into_inner())
}

/// Lo que hay guardado, sin tocar la red.
#[tauri::command]
pub fn weather_cached(cache: State<'_, WeatherCache>) -> Option<Pronostico> {
    abrir(&cache).foto(Instant::now())
}

/// Las coordenadas ya deducidas, si alguna ventana las averiguó.
#[tauri::command]
pub fn weather_place(cache: State<'_, WeatherCache>) -> Option<Value> {
    abrir(&cache).lugar.clone()
}

/// «¿Me toca pedirlo a mí?». Responde que sí a una sola ventana por vuelta.
#[tauri::command]
pub fn weather_claim(cache: State<'_, WeatherCache>) -> bool {
    abrir(&cache).reclamar(Instant::now())
}

/// Guarda lo que trajo la ventana que reclamó y se lo avisa a todas.
#[tauri::command]
pub fn weather_store(
    app: AppHandle,
    cache: State<'_, WeatherCache>,
    datos: Value,
    lugar: Option<Value>,
) -> Result<(), String> {
    let foto = {
        let mut estado = abrir(&cache);
        estado.guardar(datos, lugar, Instant::now());
        estado.foto(Instant::now())
    };

    // Fuera del candado: emitir despierta a las otras ventanas, y ninguna
    // debería quedarse esperando este mutex para contestar.
    if let Some(foto) = foto {
        app.emit("weather-updated", foto).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// El pedido falló: libera el turno así otra ventana puede intentar sin esperar
/// los treinta segundos completos.
#[tauri::command]
pub fn weather_release(cache: State<'_, WeatherCache>) {
    abrir(&cache).reclamado = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Un instante de hace tanto.
    ///
    /// `Instant::now() - duración` entra en pánico si el reloj monotónico vale
    /// menos que la duración, que es lo que pasa en una máquina recién
    /// arrancada: el test fallaba por el reloj y no por lo que prueba.
    fn hace(segundos: u64) -> Instant {
        let ahora = Instant::now();
        ahora
            .checked_sub(Duration::from_secs(segundos))
            .unwrap_or(ahora)
    }

    #[test]
    fn sin_datos_el_primero_reclama() {
        let mut estado = Estado::default();
        assert!(estado.reclamar(Instant::now()));
    }

    #[test]
    fn una_sola_ventana_pide_por_vuelta() {
        let mut estado = Estado::default();
        let ahora = Instant::now();

        assert!(estado.reclamar(ahora));
        assert!(!estado.reclamar(ahora), "la segunda ventana no debe pedir");
        assert!(!estado.reclamar(ahora), "ni la tercera");
    }

    #[test]
    fn con_datos_frescos_nadie_pide() {
        let mut estado = Estado::default();
        estado.guardar(json!({"current": {}}), None, hace(60));

        assert!(!estado.reclamar(Instant::now()));
    }

    #[test]
    fn pasada_la_vigencia_se_vuelve_a_pedir() {
        let mut estado = Estado::default();
        estado.guardar(json!({"current": {}}), None, hace(11 * 60));

        assert!(estado.reclamar(Instant::now()));
    }

    #[test]
    fn si_el_que_reclamo_no_volvio_otro_reintenta() {
        let mut estado = Estado::default();
        estado.reclamado = Some(hace(31));

        assert!(
            estado.reclamar(Instant::now()),
            "un reclamo abandonado no puede congelar el clima"
        );
    }

    #[test]
    fn el_lugar_sobrevive_a_un_guardado_sin_lugar() {
        let mut estado = Estado::default();
        let ahora = Instant::now();

        estado.guardar(json!({}), Some(json!({"lat": -34.6, "lon": -58.4})), ahora);
        estado.guardar(json!({"otro": true}), None, ahora);

        assert_eq!(estado.lugar, Some(json!({"lat": -34.6, "lon": -58.4})));
    }

    #[test]
    fn la_foto_dice_la_edad_y_si_vencio() {
        let mut estado = Estado::default();
        estado.guardar(json!({"current": {"temperature_2m": 20}}), None, hace(120));

        let foto = estado.foto(Instant::now()).expect("hay datos guardados");
        assert!((119..=121).contains(&foto.edad_segundos), "{}", foto.edad_segundos);
        assert!(!foto.vencido);
    }

    #[test]
    fn sin_datos_no_hay_foto() {
        assert!(Estado::default().foto(Instant::now()).is_none());
    }

    #[test]
    fn guardar_libera_el_turno() {
        let mut estado = Estado::default();
        let ahora = Instant::now();

        assert!(estado.reclamar(ahora));
        estado.guardar(json!({}), None, ahora);
        assert!(estado.reclamado.is_none());
    }
}
