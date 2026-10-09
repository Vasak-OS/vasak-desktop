//! El micrófono: la fuente de audio por omisión y las entradas que se pueden
//! elegir, leídas de PipeWire (vasak-desktop#182).
//!
//! # De dónde sale
//!
//! Del flujo de `pw-dump --monitor` que `audio_native::PwDumpMonitor` ya tiene
//! abierto para el volumen de salida: no suma procesos, conexiones ni sondeos.
//! Cada tanda que llega se le pasa a [`ingest`], que guarda:
//!
//! - cada **`Node`** con `media.class` `Audio/Source` (o `Audio/Source/Virtual`):
//!   su `node.name`, su `node.description`, el volumen (`channelVolumes`, en
//!   escala cúbica como el de salida) y el silencio (`mute`) de `Props`;
//! - el **`Metadata`** que se llama `default`, de donde sale
//!   `default.audio.source`: la fuente que eligió WirePlumber.
//!
//! `pw-dump` manda el objeto entero cada vez que algo cambia, así que cada uno
//! se reemplaza completo por su id; uno que se va llega como
//! `{"id": N, "info": null}`. Los monitores de las salidas no son nodos
//! aparte en PipeWire, y los filtros (`filter.smart = true`, como una
//! cancelación de ruido que WirePlumber pone delante) no se ofrecen para
//! elegir: el sonido ya pasa por ellos.
//!
//! Cuando lo que se ve cambia —el volumen o el silencio de la fuente por
//! omisión, o la lista de entradas— se avisa por un canal que el applet de
//! audio reenvía a la interfaz como `microphone-changed` y
//! `audio-input-devices-changed`.
//!
//! Sin `pw-dump` (el respaldo de `pactl` o el monitor nativo) no llega nada:
//! [`snapshot`] devuelve `None` y los comandos leen con `pactl` una vez.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use serde_json::Value;
use tokio::sync::broadcast;

use crate::structs::{AudioDevice, VolumeInfo};

/// Lo que la interfaz ve del micrófono.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioInputSnapshot {
    /// El volumen y el silencio de la fuente por omisión; `None` sin fuente.
    pub microphone: Option<VolumeInfo>,
    /// Las entradas que se pueden elegir, con la que está por omisión marcada.
    pub devices: Vec<AudioDevice>,
}

/// Una fuente de PipeWire, tal como llegó en la última tanda.
#[derive(Debug, Clone, PartialEq)]
struct SourceNode {
    name: String,
    description: String,
    /// El volumen lineal (0.0–1.0, puede pasar de 1), ya sin la escala cúbica.
    volume: f64,
    muted: bool,
}

/// Lo que se sabe del flujo de `pw-dump`.
#[derive(Debug, Default)]
pub struct AudioInputTracker {
    nodes: BTreeMap<u64, SourceNode>,
    default_source: Option<String>,
    /// Si llegó alguna tanda: la primera es el volcado entero.
    ready: bool,
}

impl AudioInputTracker {
    /// Incorpora una tanda de `pw-dump` (un arreglo de objetos).
    pub fn ingest(&mut self, batch: &Value) {
        let Some(objects) = batch.as_array() else {
            return;
        };
        self.ready = true;
        for object in objects {
            let Some(id) = object.get("id").and_then(Value::as_u64) else {
                continue;
            };
            // Un objeto que se va: `{"id": N, "info": null}`. Los metadatos no
            // traen `info`, así que la ausencia sola no alcanza.
            if object.get("info").is_some_and(Value::is_null) {
                self.nodes.remove(&id);
                continue;
            }
            match object.get("type").and_then(Value::as_str) {
                Some("PipeWire:Interface:Node") => self.ingest_node(id, object),
                Some("PipeWire:Interface:Metadata") => self.ingest_metadata(object),
                _ => {}
            }
        }
    }

    fn ingest_node(&mut self, id: u64, object: &Value) {
        let info = object.get("info");
        // Un cambio que no trae las propiedades (sólo `params`) no dice qué
        // clase de nodo es: se actualiza lo que se sabía, sin borrarlo.
        if info.and_then(|info| info.get("props")).is_none() {
            if let (Some(node), Some((volume, muted))) =
                (self.nodes.get_mut(&id), info.and_then(parse_audio_props))
            {
                node.volume = volume.unwrap_or(node.volume);
                node.muted = muted.unwrap_or(node.muted);
            }
            return;
        }
        match parse_source(object) {
            Some(node) => {
                self.nodes.insert(id, node);
            }
            // Un nodo que dejó de ser fuente (o que es un filtro).
            None => {
                self.nodes.remove(&id);
            }
        }
    }

    fn ingest_metadata(&mut self, object: &Value) {
        let name = object
            .get("props")
            .and_then(|props| props.get("metadata.name"))
            .and_then(Value::as_str);
        if name != Some("default") {
            return;
        }
        let Some(entries) = object.get("metadata").and_then(Value::as_array) else {
            return;
        };
        for entry in entries {
            if entry.get("key").and_then(Value::as_str) != Some("default.audio.source") {
                continue;
            }
            self.default_source = entry.get("value").and_then(metadata_name);
        }
    }

    /// Lo que se ve, o `None` si todavía no llegó nada de `pw-dump`.
    pub fn snapshot(&self) -> Option<AudioInputSnapshot> {
        if !self.ready {
            return None;
        }
        let default = self.default_source.as_deref();
        let microphone = self
            .nodes
            .values()
            .find(|node| Some(node.name.as_str()) == default)
            .map(|node| VolumeInfo {
                current: volume_percent(node.volume),
                min: 0,
                max: 100,
                is_muted: node.muted,
            });
        let devices = self
            .nodes
            .values()
            .map(|node| AudioDevice {
                id: node.name.clone(),
                // Como en las salidas: `name` es lo que se muestra y
                // `description` el nombre del nodo (ver `audio-device.ts`).
                name: if node.description.is_empty() {
                    node.name.clone()
                } else {
                    node.description.clone()
                },
                description: node.name.clone(),
                is_default: Some(node.name.as_str()) == default,
                volume: node.volume,
            })
            .collect();
        Some(AudioInputSnapshot {
            microphone,
            devices,
        })
    }

    /// Se olvida todo: `pw-dump` volvió a arrancar y manda el volcado entero.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// `{"name": "alsa_input…"}`, o la misma cosa como texto en versiones viejas.
fn metadata_name(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => map.get("name").and_then(Value::as_str).map(str::to_string),
        Value::String(text) => serde_json::from_str::<Value>(text)
            .ok()
            .and_then(|parsed| metadata_name(&parsed)),
        _ => None,
    }
}

/// El volumen lineal en porcentaje entero, como el de salida.
fn volume_percent(linear: f64) -> i64 {
    ((linear * 100.0).round() as i64).clamp(0, 150)
}

/// Una fuente elegible, o `None` si el nodo no lo es.
fn parse_source(object: &Value) -> Option<SourceNode> {
    let info = object.get("info")?;
    let props = info.get("props")?;
    let class = props.get("media.class").and_then(Value::as_str)?;
    if class != "Audio/Source" && class != "Audio/Source/Virtual" {
        return None;
    }
    let smart = props.get("filter.smart");
    if smart.and_then(Value::as_bool) == Some(true) || smart.and_then(Value::as_str) == Some("true")
    {
        return None;
    }
    let name = props.get("node.name").and_then(Value::as_str)?.to_string();
    let description = props
        .get("node.description")
        .or_else(|| props.get("node.nick"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let (volume, muted) = parse_audio_props(info).unwrap_or((None, None));

    Some(SourceNode {
        name,
        description,
        volume: volume.unwrap_or(0.0),
        muted: muted.unwrap_or(false),
    })
}

/// El volumen lineal y el silencio de `params.Props`, si vinieron.
fn parse_audio_props(info: &Value) -> Option<(Option<f64>, Option<bool>)> {
    let entry = info
        .get("params")?
        .get("Props")?
        .as_array()?
        .iter()
        .find(|entry| entry.get("mute").is_some() || entry.get("channelVolumes").is_some())?;
    let muted = entry.get("mute").and_then(Value::as_bool);
    // PipeWire guarda el volumen en escala cúbica: lo que se ve es la raíz.
    let volume = entry
        .get("channelVolumes")
        .and_then(Value::as_array)
        .and_then(|channels| channels.first())
        .and_then(Value::as_f64)
        .map(f64::cbrt);
    Some((volume, muted))
}

struct Shared {
    tracker: AudioInputTracker,
    /// Lo último que se avisó, para avisar sólo lo que cambió.
    last_sent: Option<AudioInputSnapshot>,
}

fn shared() -> &'static Mutex<Shared> {
    static SHARED: OnceLock<Mutex<Shared>> = OnceLock::new();
    SHARED.get_or_init(|| {
        Mutex::new(Shared {
            tracker: AudioInputTracker::default(),
            last_sent: None,
        })
    })
}

fn channel() -> &'static broadcast::Sender<AudioInputSnapshot> {
    static CHANNEL: OnceLock<broadcast::Sender<AudioInputSnapshot>> = OnceLock::new();
    CHANNEL.get_or_init(|| broadcast::channel(16).0)
}

/// Incorpora una tanda del flujo de `pw-dump` y avisa si cambió lo que se ve.
pub fn ingest(batch: &Value) {
    let mut shared = shared()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    shared.tracker.ingest(batch);
    let current = shared.tracker.snapshot();
    if current.is_some() && current != shared.last_sent {
        shared.last_sent = current.clone();
        if let Some(snapshot) = current {
            // Sin suscriptores (nadie escuchando todavía) no es un error.
            let _ = channel().send(snapshot);
        }
    }
}

/// `pw-dump` volvió a arrancar: lo que se sabía puede estar viejo.
pub fn reset() {
    let mut shared = shared()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    shared.tracker.reset();
}

/// Lo que se ve ahora, si el flujo de `pw-dump` ya mandó algo.
pub fn snapshot() -> Option<AudioInputSnapshot> {
    shared()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .tracker
        .snapshot()
}

/// Los cambios, para el applet de audio.
pub fn subscribe() -> broadcast::Receiver<AudioInputSnapshot> {
    channel().subscribe()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const MIC: &str = "alsa_input.usb-C-Media.mono-fallback";
    const HEADSET: &str = "bluez_input.00_11_22.0";

    fn source(id: u64, name: &str, description: &str, cubic: f64, mute: bool) -> Value {
        json!({
            "id": id,
            "type": "PipeWire:Interface:Node",
            "info": {
                "props": {
                    "media.class": "Audio/Source",
                    "node.name": name,
                    "node.description": description
                },
                "params": {
                    "Props": [
                        { "volume": 1.0, "mute": mute, "channelVolumes": [cubic] },
                        { "params": ["monitor.channel-volumes", false] }
                    ]
                }
            }
        })
    }

    fn defaults(source: &str) -> Value {
        json!({
            "id": 39,
            "type": "PipeWire:Interface:Metadata",
            "props": { "metadata.name": "default" },
            "metadata": [
                { "subject": 0, "key": "default.audio.sink", "type": "Spa:String:JSON",
                  "value": { "name": "alsa_output.analog-stereo" } },
                { "subject": 0, "key": "default.audio.source", "type": "Spa:String:JSON",
                  "value": { "name": source } }
            ]
        })
    }

    /// El volcado inicial: la placa al 47 % (0.104209 en cúbico) y el
    /// auricular, con la placa por omisión.
    fn dump() -> Value {
        json!([
            source(49, MIC, "USB Advanced Audio Device Mono", 0.104209, false),
            source(80, HEADSET, "Auriculares WH-1000XM4", 0.125, true),
            defaults(MIC),
            {
                "id": 52,
                "type": "PipeWire:Interface:Node",
                "info": { "props": { "media.class": "Audio/Sink", "node.name": "alsa_output.analog-stereo" } }
            }
        ])
    }

    fn tracker() -> AudioInputTracker {
        let mut tracker = AudioInputTracker::default();
        tracker.ingest(&dump());
        tracker
    }

    #[test]
    fn sin_ninguna_tanda_no_se_sabe_nada() {
        assert_eq!(AudioInputTracker::default().snapshot(), None);
    }

    #[test]
    fn refleja_el_volumen_y_el_silencio_de_la_fuente_por_omision() {
        let snapshot = tracker().snapshot().unwrap();
        assert_eq!(
            snapshot.microphone,
            Some(VolumeInfo {
                current: 47,
                min: 0,
                max: 100,
                is_muted: false
            })
        );
    }

    #[test]
    fn un_cambio_de_silencio_llega_en_la_tanda_siguiente() {
        let mut tracker = tracker();
        tracker.ingest(&json!([source(
            49,
            MIC,
            "USB Advanced Audio Device Mono",
            0.104209,
            true
        )]));
        let microphone = tracker.snapshot().unwrap().microphone.unwrap();
        assert!(microphone.is_muted);
        assert_eq!(microphone.current, 47);
    }

    #[test]
    fn cambiar_la_fuente_por_omision_cambia_el_microfono_y_la_marca() {
        let mut tracker = tracker();
        tracker.ingest(&json!([defaults(HEADSET)]));
        let snapshot = tracker.snapshot().unwrap();
        assert_eq!(
            snapshot.microphone,
            Some(VolumeInfo {
                current: 50,
                min: 0,
                max: 100,
                is_muted: true
            })
        );
        let marked: Vec<_> = snapshot
            .devices
            .iter()
            .filter(|device| device.is_default)
            .map(|device| device.id.as_str())
            .collect();
        assert_eq!(marked, [HEADSET]);
    }

    #[test]
    fn las_entradas_van_con_su_nombre_visible_y_el_del_nodo_como_id() {
        let devices = tracker().snapshot().unwrap().devices;
        assert_eq!(devices.len(), 2, "la salida no es una entrada");
        assert_eq!(devices[0].id, MIC);
        assert_eq!(devices[0].name, "USB Advanced Audio Device Mono");
        assert_eq!(devices[0].description, MIC);
        assert!(devices[0].is_default);
        assert!(!devices[1].is_default);
    }

    #[test]
    fn una_fuente_que_se_va_desaparece_y_sin_ella_no_hay_microfono() {
        let mut tracker = tracker();
        tracker.ingest(&json!([{ "id": 49, "info": null }]));
        let snapshot = tracker.snapshot().unwrap();
        assert_eq!(snapshot.microphone, None);
        assert_eq!(snapshot.devices.len(), 1);
    }

    #[test]
    fn un_cambio_sin_propiedades_actualiza_sin_borrar() {
        let mut tracker = tracker();
        tracker.ingest(&json!([{
            "id": 49,
            "type": "PipeWire:Interface:Node",
            "info": { "params": { "Props": [{ "mute": true, "channelVolumes": [0.125] }] } }
        }]));
        let microphone = tracker.snapshot().unwrap().microphone.unwrap();
        assert!(microphone.is_muted);
        assert_eq!(microphone.current, 50);

        tracker.ingest(
            &json!([{ "id": 49, "type": "PipeWire:Interface:Node", "info": { "params": {} } }]),
        );
        assert_eq!(tracker.snapshot().unwrap().devices.len(), 2);
    }

    #[test]
    fn un_filtro_no_es_una_entrada_que_se_pueda_elegir() {
        let mut tracker = tracker();
        let mut filter = source(90, "rnnoise_source", "Cancelación de ruido", 1.0, false);
        filter["info"]["props"]["filter.smart"] = json!(true);
        tracker.ingest(&json!([filter]));
        assert_eq!(tracker.snapshot().unwrap().devices.len(), 2);
    }

    #[test]
    fn el_nombre_por_omision_tambien_se_lee_si_viene_como_texto() {
        let mut tracker = tracker();
        let mut metadata = defaults(HEADSET);
        metadata["metadata"][1]["value"] = json!(format!("{{\"name\": \"{HEADSET}\"}}"));
        tracker.ingest(&json!([metadata]));
        assert!(tracker.snapshot().unwrap().devices[1].is_default);
    }

    #[test]
    fn otros_metadatos_no_tocan_la_fuente_por_omision() {
        let mut tracker = tracker();
        tracker.ingest(&json!([{
            "id": 42,
            "type": "PipeWire:Interface:Metadata",
            "props": { "metadata.name": "route-settings" },
            "metadata": [{ "subject": 0, "key": "default.audio.source", "value": { "name": HEADSET } }]
        }]));
        assert!(tracker.snapshot().unwrap().devices[0].is_default);
    }

    #[test]
    fn al_reiniciar_se_olvida_lo_que_se_sabia() {
        let mut tracker = tracker();
        tracker.reset();
        assert_eq!(tracker.snapshot(), None);
    }
}
