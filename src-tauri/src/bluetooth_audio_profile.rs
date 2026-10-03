//! El perfil de audio activo de cada dispositivo Bluetooth, leído de PipeWire.
//!
//! Lo pide la vista radial del Bluetooth (vasak-desktop#132): un satélite con
//! «A2DP · AAC» o «Manos libres · mSBC». BlueZ no es la fuente fiel —sus
//! `uuids` dicen qué perfiles *soporta* el dispositivo, no cuál está activo, y
//! con el auricular en modo llamada no publica ningún transporte—, así que se
//! lee de quien lo elige: PipeWire/WirePlumber.
//!
//! # De dónde sale
//!
//! Del flujo de `pw-dump --monitor` que `audio_native::PwDumpMonitor` ya tiene
//! corriendo para el volumen: no suma ningún proceso ni conexión. Cada tanda
//! que llega se le pasa a [`ingest`], que mira los objetos con
//! `device.api = "bluez5"`:
//!
//! - el **`Device`** trae `api.bluez5.address` (la MAC, que es como se cruza con
//!   el `address` del complemento de Bluetooth) y el parámetro `Profile` con el
//!   perfil activo (`name`) y su nombre legible (`description`);
//! - cada **`Node`** trae `api.bluez5.address`, `api.bluez5.profile`
//!   (`a2dp-sink`, `headset-head-unit`…) y `api.bluez5.codec` (SBC, AAC, LDAC,
//!   mSBC…), que es lo único que da el códec.
//!
//! Un objeto que se va llega como `{"id": N, "info": null}`: por eso se guarda
//! cada uno por su id, y la MAC de un id que desaparece también se avisa. Un
//! cambio de perfil recrea los nodos, así que llega solo.
//!
//! Los nombres de perfil cambiaron entre versiones de PipeWire
//! (`a2dp-sink-aac`, `headset-head-unit-msbc`…): no se traducen acá. Va el
//! nombre tal cual y la descripción que da WirePlumber, y la interfaz elige qué
//! mostrar.
//!
//! El complemento `tauri-plugin-bluetooth-manager` sigue hablando sólo con
//! BlueZ: meterle PipeWire obligaría a cada aplicación que lo use a abrir su
//! propia conexión para un dato que el escritorio ya tiene.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use serde_json::Value;
use tokio::sync::broadcast;

/// El perfil activo de un dispositivo, tal como lo manda la interfaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BluetoothAudioProfile {
    /// La MAC, en mayúsculas.
    pub address: String,
    /// El nombre de PipeWire: `a2dp-sink`, `headset-head-unit`… `None` si el
    /// perfil está apagado o no hay audio.
    pub profile: Option<String>,
    /// El nombre legible que da WirePlumber («High Fidelity Playback (A2DP
    /// Sink, codec AAC)»), si lo hay.
    pub description: Option<String>,
    /// El códec del transporte: `SBC`, `AAC`, `LDAC`, `mSBC`…
    pub codec: Option<String>,
}

/// Lo que se sabe de un objeto de PipeWire de Bluetooth.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    Device {
        address: String,
        profile: Option<String>,
        description: Option<String>,
    },
    Node {
        address: String,
        profile: Option<String>,
        codec: Option<String>,
    },
}

impl Entry {
    fn address(&self) -> &str {
        match self {
            Entry::Device { address, .. } | Entry::Node { address, .. } => address,
        }
    }
}

/// Los objetos de Bluetooth que se vieron, por id de PipeWire.
#[derive(Debug, Default)]
pub struct ProfileTracker {
    by_id: HashMap<u64, Entry>,
}

/// Un texto de las propiedades, si no está vacío.
fn text(props: &Value, key: &str) -> Option<String> {
    props
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// Un perfil `off` es que no hay audio: se informa como ninguno.
fn active(profile: Option<String>) -> Option<String> {
    profile.filter(|name| name != "off")
}

/// Lo que dice un objeto de `pw-dump`, si es de Bluetooth y tiene MAC.
fn entry_of(object: &Value) -> Option<Entry> {
    let info = object.get("info")?;
    let props = info.get("props")?;
    if props.get("device.api").and_then(Value::as_str) != Some("bluez5") {
        return None;
    }
    let address = text(props, "api.bluez5.address")?.to_uppercase();

    match object.get("type").and_then(Value::as_str)? {
        "PipeWire:Interface:Device" => {
            let current = info
                .get("params")
                .and_then(|params| params.get("Profile"))
                .and_then(Value::as_array)
                .and_then(|profiles| profiles.first());
            let profile = current.and_then(|p| text(p, "name"));
            let description = current.and_then(|p| text(p, "description"));
            Some(Entry::Device {
                address,
                profile: active(profile),
                description,
            })
        }
        "PipeWire:Interface:Node" => Some(Entry::Node {
            address,
            profile: active(text(props, "api.bluez5.profile")),
            codec: text(props, "api.bluez5.codec"),
        }),
        _ => None,
    }
}

impl ProfileTracker {
    /// Suma una tanda de `pw-dump` y devuelve las MAC cuyo perfil cambió.
    pub fn apply(&mut self, batch: &Value) -> BTreeSet<String> {
        let mut touched = BTreeSet::new();
        let Some(objects) = batch.as_array() else {
            return touched;
        };

        for object in objects {
            let Some(id) = object.get("id").and_then(Value::as_u64) else {
                continue;
            };
            let removed = object.get("info").is_none_or(Value::is_null);
            if removed {
                if let Some(old) = self.by_id.remove(&id) {
                    touched.insert(old.address().to_owned());
                }
                continue;
            }
            // Un objeto que cambia sin dejar de existir puede llegar sin las
            // propiedades de Bluetooth (sólo `params`): se queda el que había.
            let Some(entry) = entry_of(object) else {
                continue;
            };
            let address = entry.address().to_owned();
            if self.by_id.get(&id) != Some(&entry) {
                if let Some(old) = self.by_id.insert(id, entry) {
                    touched.insert(old.address().to_owned());
                }
                touched.insert(address);
            }
        }

        // Una MAC que cambió de valor y volvió al mismo no hace falta avisarla,
        // pero tampoco molesta: la interfaz compara antes de redibujar.
        touched
    }

    /// El perfil de un dispositivo, juntando lo del `Device` y lo de sus nodos.
    ///
    /// `None` si PipeWire no tiene ningún objeto de esa MAC: el dispositivo no
    /// es de audio, o no está conectado.
    pub fn profile_of(&self, address: &str) -> Option<BluetoothAudioProfile> {
        let address = address.to_uppercase();
        let mut found = false;
        let mut profile = None;
        let mut description = None;
        let mut node_profile = None;
        let mut codec = None;

        // Por id, para que con dos nodos (salida y micrófono) gane siempre el
        // mismo: el de menor id, que es el primero que se creó.
        let mut entries: Vec<_> = self
            .by_id
            .iter()
            .filter(|(_, entry)| entry.address() == address)
            .collect();
        entries.sort_by_key(|(id, _)| **id);

        for (_, entry) in entries {
            found = true;
            match entry {
                Entry::Device {
                    profile: p,
                    description: d,
                    ..
                } => {
                    profile = p.clone();
                    description = d.clone();
                }
                Entry::Node {
                    profile: p,
                    codec: c,
                    ..
                } => {
                    if node_profile.is_none() {
                        node_profile = p.clone();
                    }
                    if codec.is_none() {
                        codec = c.clone();
                    }
                }
            }
        }

        found.then(|| BluetoothAudioProfile {
            address,
            profile: profile.or(node_profile),
            description,
            codec,
        })
    }
}

fn tracker() -> &'static Mutex<ProfileTracker> {
    static TRACKER: OnceLock<Mutex<ProfileTracker>> = OnceLock::new();
    TRACKER.get_or_init(|| Mutex::new(ProfileTracker::default()))
}

fn channel() -> &'static broadcast::Sender<BluetoothAudioProfile> {
    static CHANNEL: OnceLock<broadcast::Sender<BluetoothAudioProfile>> = OnceLock::new();
    CHANNEL.get_or_init(|| broadcast::channel(32).0)
}

/// Suma una tanda de `pw-dump --monitor` y avisa los perfiles que cambiaron.
///
/// Un dispositivo que se fue se avisa con `profile`, `description` y `codec`
/// en `None`: la interfaz saca el satélite.
pub fn ingest(batch: &Value) {
    let changed: Vec<BluetoothAudioProfile> = {
        let Ok(mut tracker) = tracker().lock() else {
            return;
        };
        let touched = tracker.apply(batch);
        touched
            .into_iter()
            .map(|address| {
                tracker
                    .profile_of(&address)
                    .unwrap_or(BluetoothAudioProfile {
                        address,
                        profile: None,
                        description: None,
                        codec: None,
                    })
            })
            .collect()
    };
    for profile in changed {
        // Sin nadie escuchando, `send` falla: no es un error.
        let _ = channel().send(profile);
    }
}

/// Los cambios de perfil, para reenviarlos a la interfaz como evento.
pub fn subscribe() -> broadcast::Receiver<BluetoothAudioProfile> {
    channel().subscribe()
}

/// El perfil de una MAC ahora: para cuando el applet se abre y todavía no
/// llegó ningún cambio.
#[tauri::command]
pub async fn get_bluetooth_audio_profile(address: String) -> Option<BluetoothAudioProfile> {
    tracker().lock().ok()?.profile_of(&address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const MAC: &str = "08:92:CC:7C:37:A1";

    fn device(id: u64, profile: &str, description: &str) -> Value {
        json!({
            "id": id,
            "type": "PipeWire:Interface:Device",
            "info": {
                "props": { "device.api": "bluez5", "api.bluez5.address": MAC.to_lowercase() },
                "params": { "Profile": [ { "index": 1, "name": profile, "description": description } ] }
            }
        })
    }

    fn node(id: u64, profile: &str, codec: &str) -> Value {
        json!({
            "id": id,
            "type": "PipeWire:Interface:Node",
            "info": {
                "props": {
                    "device.api": "bluez5",
                    "media.class": "Audio/Sink",
                    "api.bluez5.address": MAC,
                    "api.bluez5.profile": profile,
                    "api.bluez5.codec": codec
                }
            }
        })
    }

    #[test]
    fn junta_el_perfil_del_dispositivo_con_el_codec_del_nodo() {
        let mut tracker = ProfileTracker::default();
        let touched = tracker.apply(&json!([
            device(
                40,
                "a2dp-sink",
                "High Fidelity Playback (A2DP Sink, codec AAC)"
            ),
            node(41, "a2dp-sink", "AAC"),
        ]));

        assert_eq!(
            touched.into_iter().collect::<Vec<_>>(),
            vec![MAC.to_owned()]
        );
        assert_eq!(
            tracker.profile_of(&MAC.to_lowercase()),
            Some(BluetoothAudioProfile {
                address: MAC.to_owned(),
                profile: Some("a2dp-sink".into()),
                description: Some("High Fidelity Playback (A2DP Sink, codec AAC)".into()),
                codec: Some("AAC".into()),
            })
        );
    }

    #[test]
    fn el_cambio_a_manos_libres_recrea_el_nodo_y_llega_solo() {
        let mut tracker = ProfileTracker::default();
        tracker.apply(&json!([
            device(40, "a2dp-sink", "A2DP"),
            node(41, "a2dp-sink", "AAC")
        ]));

        let touched = tracker.apply(&json!([
            { "id": 41, "info": null },
            device(40, "headset-head-unit", "Headset Head Unit (HSP/HFP, codec mSBC)"),
            node(52, "headset-head-unit", "mSBC"),
        ]));

        assert!(touched.contains(MAC));
        let profile = tracker.profile_of(MAC).expect("sigue conectado");
        assert_eq!(profile.profile.as_deref(), Some("headset-head-unit"));
        assert_eq!(profile.codec.as_deref(), Some("mSBC"));
    }

    #[test]
    fn sin_objetos_de_esa_mac_no_hay_perfil() {
        let mut tracker = ProfileTracker::default();
        tracker.apply(&json!([device(40, "a2dp-sink", "A2DP")]));
        tracker.apply(&json!([{ "id": 40, "info": null }]));

        assert_eq!(tracker.profile_of(MAC), None);
    }

    #[test]
    fn el_perfil_off_es_ninguno() {
        let mut tracker = ProfileTracker::default();
        tracker.apply(&json!([device(40, "off", "Off")]));

        assert_eq!(tracker.profile_of(MAC).and_then(|p| p.profile), None);
    }

    #[test]
    fn sin_el_dispositivo_el_perfil_sale_del_nodo() {
        let mut tracker = ProfileTracker::default();
        tracker.apply(&json!([node(41, "a2dp-sink-aac", "AAC")]));

        let profile = tracker.profile_of(MAC).expect("hay nodo");
        assert_eq!(profile.profile.as_deref(), Some("a2dp-sink-aac"));
        assert_eq!(profile.description, None);
    }

    #[test]
    fn lo_que_no_es_bluetooth_no_cuenta_y_lo_repetido_no_avisa() {
        let mut tracker = ProfileTracker::default();
        let other = json!([{
            "id": 7,
            "type": "PipeWire:Interface:Node",
            "info": { "props": { "device.api": "alsa", "media.class": "Audio/Sink" } }
        }]);
        assert!(tracker.apply(&other).is_empty());

        tracker.apply(&json!([node(41, "a2dp-sink", "AAC")]));
        assert!(tracker
            .apply(&json!([node(41, "a2dp-sink", "AAC")]))
            .is_empty());
    }

    #[test]
    fn un_cambio_sin_propiedades_no_borra_lo_que_habia() {
        let mut tracker = ProfileTracker::default();
        tracker.apply(&json!([node(41, "a2dp-sink", "AAC")]));
        tracker.apply(
            &json!([{ "id": 41, "type": "PipeWire:Interface:Node", "info": { "params": {} } }]),
        );

        assert_eq!(
            tracker.profile_of(MAC).and_then(|p| p.codec).as_deref(),
            Some("AAC")
        );
    }

    #[test]
    fn lo_que_no_es_una_lista_no_rompe_nada() {
        let mut tracker = ProfileTracker::default();
        assert!(tracker.apply(&json!({ "id": 1 })).is_empty());
    }
}
