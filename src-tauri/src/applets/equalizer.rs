//! El ecualizador de sistema, tal como lo publica WirePlumber.
//!
//! El filtro y su estado son de `vasak-wireplumber-modules` (#10, #12): un
//! servicio `org.vasak.Equalizer` en el bus de la sesión, objeto
//! `/org/vasak/Equalizer`, interfaz `org.vasak.Equalizer1`. Acá no se decide
//! nada del sonido: se lee el estado, se manda lo que la persona tocó y se
//! avisa a la interfaz cuando algo cambia (`equalizer-changed`).
//!
//! # Cuando no está
//!
//! El nombre lo tiene WirePlumber mientras corre, y puede no estar: sin el
//! paquete, mientras arranca o si se cayó. Entonces el estado sale con
//! `service` en falso y la interfaz dibuja el ecualizador **no disponible**,
//! nunca roto. Se vigila `NameOwnerChanged` para avisar en cuanto aparezca o
//! desaparezca, sin sondear.
//!
//! # Los cambios
//!
//! Llegan por `PropertiesChanged`, sólo con lo que cambió. En lugar de juntar
//! las piezas, cada aviso vuelve a leer las propiedades: son diez valores
//! chicos, y así el estado que llega a la interfaz es siempre uno entero y
//! coherente.

use super::Applet;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use zbus::{proxy, Connection, MatchRule, MessageStream};

/// Dónde vive el servicio.
pub const SERVICE: &str = "org.vasak.Equalizer";
pub const PATH: &str = "/org/vasak/Equalizer";
pub const INTERFACE: &str = "org.vasak.Equalizer1";

/// El evento que recibe la interfaz con el estado entero.
pub const CHANGED_EVENT: &str = "equalizer-changed";

/// El contrato, copiado del README de vasak-wireplumber-modules.
#[proxy(
    interface = "org.vasak.Equalizer1",
    default_service = "org.vasak.Equalizer",
    default_path = "/org/vasak/Equalizer"
)]
pub trait Equalizer {
    fn set_gain(&self, band: u32, gain: f64) -> zbus::Result<()>;
    fn set_gains(&self, gains: &[f64]) -> zbus::Result<()>;
    fn set_preset(&self, preset: &str) -> zbus::Result<()>;
    fn set_enabled(&self, enabled: bool) -> zbus::Result<()>;

    #[zbus(property)]
    fn frequencies(&self) -> zbus::Result<Vec<f64>>;
    #[zbus(property)]
    fn gain_range(&self) -> zbus::Result<(f64, f64)>;
    #[zbus(property)]
    fn presets(&self) -> zbus::Result<Vec<String>>;
    #[zbus(property)]
    fn preset(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn gains(&self) -> zbus::Result<Vec<f64>>;
    #[zbus(property)]
    fn enabled(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn available(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn saved(&self) -> zbus::Result<bool>;
}

/// Lo que la interfaz necesita para dibujar el ecualizador.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EqualizerState {
    /// Si el servicio está en el bus. Sin él, lo demás son los valores vacíos.
    pub service: bool,
    pub frequencies: Vec<f64>,
    /// `[mínimo, máximo]` en dB.
    pub range: (f64, f64),
    pub presets: Vec<String>,
    pub preset: String,
    pub gains: Vec<f64>,
    pub enabled: bool,
    /// Si el filtro está en PipeWire (falso hasta reiniciarlo tras instalar).
    pub available: bool,
    pub saved: bool,
}

impl EqualizerState {
    /// El estado cuando el servicio no está en el bus.
    pub fn missing() -> Self {
        Self {
            service: false,
            frequencies: Vec::new(),
            range: (-12.0, 12.0),
            presets: Vec::new(),
            preset: String::new(),
            gains: Vec::new(),
            enabled: false,
            available: false,
            saved: true,
        }
    }
}

/// Si una banda y una ganancia tienen sentido antes de mandarlas.
///
/// El servicio contesta `InvalidArgs` a lo que no; esto evita el viaje y deja
/// un error que dice qué estuvo mal, en vez del de D-Bus.
pub fn check_gain(band: u32, gain: f64, bands: usize, range: (f64, f64)) -> Result<(), String> {
    if (band as usize) >= bands {
        return Err(format!("no hay banda {band}: son {bands}"));
    }
    if !gain.is_finite() || gain < range.0 || gain > range.1 {
        return Err(format!(
            "la ganancia {gain} está fuera de [{}, {}]",
            range.0, range.1
        ));
    }
    Ok(())
}

async fn service_present(conn: &Connection) -> bool {
    match zbus::fdo::DBusProxy::new(conn).await {
        Ok(dbus) => match zbus::names::BusName::try_from(SERVICE) {
            Ok(name) => dbus.name_has_owner(name).await.unwrap_or(false),
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// El estado entero, o el de «no está».
pub async fn read_state(conn: &Connection) -> EqualizerState {
    if !service_present(conn).await {
        return EqualizerState::missing();
    }
    let Ok(proxy) = EqualizerProxy::builder(conn)
        .cache_properties(zbus::CacheProperties::No)
        .build()
        .await
    else {
        return EqualizerState::missing();
    };
    let missing = EqualizerState::missing();
    EqualizerState {
        service: true,
        frequencies: proxy.frequencies().await.unwrap_or_default(),
        range: proxy.gain_range().await.unwrap_or(missing.range),
        presets: proxy.presets().await.unwrap_or_default(),
        preset: proxy.preset().await.unwrap_or_default(),
        gains: proxy.gains().await.unwrap_or_default(),
        enabled: proxy.enabled().await.unwrap_or(false),
        available: proxy.available().await.unwrap_or(false),
        saved: proxy.saved().await.unwrap_or(true),
    }
}

/// Un proxy sin caché: cada llamada va al servicio.
pub async fn proxy(conn: &Connection) -> Result<EqualizerProxy<'static>, String> {
    EqualizerProxy::builder(conn)
        .cache_properties(zbus::CacheProperties::No)
        .build()
        .await
        .map_err(|e| e.to_string())
}

pub struct EqualizerApplet;

#[async_trait]
impl Applet for EqualizerApplet {
    fn name(&self) -> &'static str {
        "equalizer"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::session().await?;
        watch(&app, &conn).await
    }
}

/// Avisa a la interfaz cada vez que el servicio cambia, aparece o se va.
pub async fn watch(app: &AppHandle, conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let properties = MatchRule::builder()
        .msg_type(zbus::MessageType::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .path(PATH)?
        .arg(0, INTERFACE)?
        .build();
    let owner = MatchRule::builder()
        .msg_type(zbus::MessageType::Signal)
        .sender("org.freedesktop.DBus")?
        .interface("org.freedesktop.DBus")?
        .member("NameOwnerChanged")?
        .arg(0, SERVICE)?
        .build();

    let mut changes = MessageStream::for_match_rule(properties, conn, Some(64)).await?;
    let mut owners = MessageStream::for_match_rule(owner, conn, Some(8)).await?;

    let _ = app.emit(CHANGED_EVENT, read_state(conn).await);
    loop {
        tokio::select! {
            message = changes.next() => if message.is_none() { break },
            message = owners.next() => if message.is_none() { break },
        }
        let _ = app.emit(CHANGED_EVENT, read_state(conn).await);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Contra el servicio de verdad en un bus **privado**, nunca el de la
    /// sesión: `tests/equalizer/fake-service.py` hace de WirePlumber.
    ///
    /// ```sh
    /// dbus-run-session -- sh -c 'python3 tests/equalizer/fake-service.py & sleep 1; \
    ///   cargo test --lib equalizer -- --ignored'
    /// ```
    #[tokio::test(flavor = "current_thread")]
    #[ignore = "necesita un bus privado con el servicio de prueba"]
    async fn lee_y_manda_contra_un_bus_privado() {
        let conn = Connection::session()
            .await
            .expect("bus de la sesión (privado)");
        let state = read_state(&conn).await;
        assert!(state.service, "el servicio de prueba no está en el bus");
        assert_eq!(state.frequencies.len(), 10);
        assert_eq!(state.range, (-12.0, 12.0));
        assert_eq!(state.presets.len(), 8);

        let eq = proxy(&conn).await.unwrap();
        eq.set_preset("rock").await.unwrap();
        assert_eq!(read_state(&conn).await.preset, "rock");

        eq.set_gain(2, 4.5).await.unwrap();
        let state = read_state(&conn).await;
        assert_eq!(state.preset, "custom");
        assert_eq!(state.gains[2], 4.5);
    }

    #[test]
    fn sin_el_servicio_el_estado_dice_que_no_esta() {
        let state = EqualizerState::missing();
        assert!(!state.service);
        assert!(!state.available);
        assert!(state.gains.is_empty());
        assert_eq!(state.range, (-12.0, 12.0));
    }

    #[test]
    fn el_estado_viaja_con_los_nombres_de_la_interfaz() {
        let json = serde_json::to_value(EqualizerState::missing()).unwrap();
        for key in [
            "service",
            "frequencies",
            "range",
            "presets",
            "preset",
            "gains",
            "enabled",
            "available",
            "saved",
        ] {
            assert!(json.get(key).is_some(), "falta {key}");
        }
        assert_eq!(json["range"], serde_json::json!([-12.0, 12.0]));
    }

    #[test]
    fn una_banda_o_una_ganancia_fuera_de_lugar_no_se_manda() {
        let range = (-12.0, 12.0);
        assert!(check_gain(0, 0.0, 10, range).is_ok());
        assert!(check_gain(9, 12.0, 10, range).is_ok());
        assert!(check_gain(10, 0.0, 10, range).is_err());
        assert!(check_gain(0, 12.5, 10, range).is_err());
        assert!(check_gain(0, f64::NAN, 10, range).is_err());
    }
}
