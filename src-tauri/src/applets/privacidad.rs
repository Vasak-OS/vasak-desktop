//! Quién te mira y quién te escucha.
//!
//! El panel tenía un indicador de micrófono **silenciado**, que es la
//! información contraria a la que hace falta para saber si alguien te está
//! grabando. Esto es lo otro: aparece cuando una aplicación abre la cámara o
//! el micrófono, y dice cuál.
//!
//! Las dos mitades se detectan distinto porque el sistema las expone distinto.
//!
//! **La cámara, por el nodo del dispositivo.** El kernel emite `IN_OPEN` e
//! `IN_CLOSE` sobre `/dev/videoN`, así que no hace falta sondear nada: se
//! espera el evento y recién ahí se recorre `/proc` para ver quién lo tiene.
//!
//! **El micrófono, por PipeWire.** Un micro en uso es un nodo
//! `Stream/Input/Audio` en `running`. Qué hay del otro lado importa: grabar el
//! eco del altavoz —un grabador de pantalla— produce un nodo de la misma
//! clase, y decir «te están escuchando» por eso sería mentir.
//!
//! Y un indicador informa, no impide. Esto no ve procesos de otro usuario ni
//! detiene a nadie; es un semáforo, no una cerradura.

use super::Applet;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

/// El proceso que puentea la cámara del teléfono hacia el dispositivo virtual.
///
/// Cuando usás el celular como webcam, scrcpy abre el loopback para **meterle**
/// fotogramas. Desde afuera eso es idéntico a una aplicación que te mira, y
/// encima lo abre en lectura y escritura, así que la bandera del descriptor no
/// lo delata. Lo que sí lo delata es de quién es hijo: el daemon de
/// vasak-connect es el que lo lanza.
const PUENTE_DE_WEBCAM: &str = "vasak-connect";

/// El evento que escucha el panel.
const EVENTO: &str = "privacidad-en-uso";

/// Una aplicación usando un dispositivo.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Uso {
    /// Con qué nombre se presenta la aplicación.
    pub aplicacion: String,
    /// Qué está usando: el nodo de vídeo, o el micrófono del que graba.
    pub detalle: String,
}

/// Lo que el panel necesita saber. Vacío es «nadie».
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct EnUso {
    pub camara: Vec<Uso>,
    pub microfono: Vec<Uso>,
}

// ─── L a   c á m a r a ───────────────────────────────────────────────────────

/// ¿El destino de un descriptor es un nodo de vídeo?
///
/// Se pide que lo que sigue a `/dev/video` sean dígitos y nada más: hay
/// dispositivos como `/dev/videoN-metadata` en algunos kernels, y sobre todo
/// hay que no confundirse con cualquier ruta que empiece parecido.
pub fn es_nodo_de_video(destino: &str) -> bool {
    destino
        .strip_prefix("/dev/video")
        .is_some_and(|resto| !resto.is_empty() && resto.bytes().all(|b| b.is_ascii_digit()))
}

/// ¿Ese descriptor está abierto sólo para escribir?
///
/// La línea viene de `/proc/PID/fdinfo/N` y trae el valor en octal:
/// `0100000` es lectura y `0100001` escritura. Quien sólo escribe no te está
/// mirando —le está dando de comer al dispositivo.
pub fn solo_escribe(linea_de_flags: &str) -> bool {
    let Some(valor) = linea_de_flags.split_whitespace().nth(1) else {
        return false;
    };
    let Ok(flags) = u32::from_str_radix(valor, 8) else {
        return false;
    };
    // O_ACCMODE: 0 lectura, 1 escritura, 2 las dos.
    flags & 0o3 == 1
}

/// El PID del padre, sacado de `/proc/PID/stat`.
///
/// El nombre del proceso va entre paréntesis y puede tener espacios —y hasta
/// paréntesis— adentro, así que se corta por el **último** `)` y no por el
/// primer espacio. Después de ahí los campos son el estado y el padre.
pub fn padre_de(stat: &str) -> Option<u32> {
    let (_, resto) = stat.rsplit_once(')')?;
    resto.split_whitespace().nth(1)?.parse().ok()
}

/// Lee `comm` de un proceso, sin el salto de línea.
fn comm_de(ruta: &Path) -> Option<String> {
    std::fs::read_to_string(ruta.join("comm"))
        .ok()
        .map(|texto| texto.trim().to_string())
}

/// ¿Este proceso es el puente que le escribe a la cámara virtual?
fn es_el_puente_del_telefono(ruta: &Path) -> bool {
    let Ok(stat) = std::fs::read_to_string(ruta.join("stat")) else {
        return false;
    };
    let Some(padre) = padre_de(&stat) else {
        return false;
    };
    comm_de(&PathBuf::from(format!("/proc/{padre}"))).as_deref() == Some(PUENTE_DE_WEBCAM)
}

/// Recorre `/proc` y arma la lista de quién tiene una cámara abierta.
///
/// Esto es caro —un `readlink` por descriptor de cada proceso— y por eso sólo
/// se llama cuando inotify avisó que algo pasó, nunca en bucle.
pub fn quien_usa_la_camara() -> Vec<Uso> {
    let mut usos = Vec::new();

    let Ok(procesos) = std::fs::read_dir("/proc") else {
        return usos;
    };

    for proceso in procesos.flatten() {
        let nombre = proceso.file_name();
        if !nombre.to_string_lossy().bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }

        let ruta = proceso.path();
        let Ok(descriptores) = std::fs::read_dir(ruta.join("fd")) else {
            // Procesos de otro usuario, o que se murieron mientras mirábamos.
            continue;
        };

        // Un mismo proceso abre la cámara y su nodo de metadatos: es una sola
        // aplicación mirándote, no dos.
        let mut dispositivos: BTreeSet<String> = BTreeSet::new();

        for descriptor in descriptores.flatten() {
            let Ok(destino) = std::fs::read_link(descriptor.path()) else {
                continue;
            };
            let Some(destino) = destino.to_str() else {
                continue;
            };
            if !es_nodo_de_video(destino) {
                continue;
            }

            let fdinfo = ruta.join("fdinfo").join(descriptor.file_name());
            if let Ok(texto) = std::fs::read_to_string(&fdinfo) {
                if texto
                    .lines()
                    .find(|linea| linea.starts_with("flags:"))
                    .is_some_and(solo_escribe)
                {
                    continue;
                }
            }

            dispositivos.insert(destino.to_string());
        }

        if dispositivos.is_empty() || es_el_puente_del_telefono(&ruta) {
            continue;
        }

        let Some(aplicacion) = comm_de(&ruta) else {
            continue;
        };

        usos.push(Uso {
            aplicacion,
            detalle: dispositivos.into_iter().collect::<Vec<_>>().join(", "),
        });
    }

    usos.sort();
    usos.dedup();
    usos
}

// ─── E l   m i c r ó f o n o ─────────────────────────────────────────────────

/// Quién está grabando de un micrófono de verdad, según una foto de `pw-dump`.
///
/// La tentación es mirar `target.object` del flujo, y es justamente lo que
/// falla: pidiéndole a `pw-record` el monitor de una salida, esa propiedad
/// decía «monitor» mientras el enlace real iba al micrófono. Lo que no miente
/// es el nodo del otro lado del enlace: `Audio/Source` es un micrófono y
/// `Audio/Sink` es el eco de lo que está sonando.
pub fn microfonos_en_uso(dump: &Value) -> Vec<Uso> {
    let Some(objetos) = dump.as_array() else {
        return Vec::new();
    };

    let mut clase_de: HashMap<i64, String> = HashMap::new();
    let mut nombre_de: HashMap<i64, String> = HashMap::new();
    let mut enlaces: Vec<(i64, i64)> = Vec::new();
    let mut flujos: Vec<(i64, String)> = Vec::new();

    for objeto in objetos {
        match objeto.get("type").and_then(Value::as_str) {
            Some("PipeWire:Interface:Node") => {
                let Some(id) = objeto.get("id").and_then(Value::as_i64) else {
                    continue;
                };
                let props = objeto.pointer("/info/props");
                let leer = |clave: &str| {
                    props
                        .and_then(|p| p.get(clave))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                };

                let clase = leer("media.class").unwrap_or_default();
                if let Some(nombre) = leer("node.description").or_else(|| leer("node.name")) {
                    nombre_de.insert(id, nombre);
                }

                if clase == "Stream/Input/Audio"
                    && objeto.pointer("/info/state").and_then(Value::as_str) == Some("running")
                {
                    let aplicacion = leer("application.name")
                        .or_else(|| leer("node.name"))
                        .unwrap_or_else(|| id.to_string());
                    flujos.push((id, aplicacion));
                }

                clase_de.insert(id, clase);
            }
            Some("PipeWire:Interface:Link") => {
                let entrada = objeto
                    .pointer("/info/input-node-id")
                    .and_then(Value::as_i64);
                let salida = objeto
                    .pointer("/info/output-node-id")
                    .and_then(Value::as_i64);
                if let (Some(entrada), Some(salida)) = (entrada, salida) {
                    enlaces.push((entrada, salida));
                }
            }
            _ => {}
        }
    }

    let mut usos: Vec<Uso> = Vec::new();

    for (id, aplicacion) in flujos {
        let fuente = enlaces
            .iter()
            .filter(|(entrada, _)| *entrada == id)
            .map(|(_, salida)| *salida)
            .find(|salida| clase_de.get(salida).map(String::as_str) == Some("Audio/Source"));

        let Some(fuente) = fuente else {
            // Sin enlace a una fuente real: es el eco del altavoz, o un flujo
            // que todavía no se conectó a nada.
            continue;
        };

        usos.push(Uso {
            aplicacion,
            detalle: nombre_de.get(&fuente).cloned().unwrap_or_default(),
        });
    }

    usos.sort();
    usos.dedup();
    usos
}

/// Pide una foto del grafo de PipeWire y saca de ahí los micrófonos en uso.
async fn mirar_los_microfonos() -> Vec<Uso> {
    let salida = tokio::process::Command::new("pw-dump").output().await;

    let Ok(salida) = salida else {
        return Vec::new();
    };

    serde_json::from_slice::<Value>(&salida.stdout)
        .map(|dump| microfonos_en_uso(&dump))
        .unwrap_or_default()
}

// ─── E l   a p p l e t ───────────────────────────────────────────────────────

pub struct PrivacidadApplet;

/// Lo último que se le contó al panel, para no repetirlo.
type Ultimo = Arc<Mutex<EnUso>>;

/// Publica el estado si cambió algo.
async fn anunciar(app: &AppHandle, ultimo: &Ultimo, nuevo: EnUso) {
    let mut guardado = ultimo.lock().await;
    if *guardado == nuevo {
        return;
    }
    *guardado = nuevo.clone();
    drop(guardado);

    if let Err(error) = app.emit(EVENTO, &nuevo) {
        log::error!("No se pudo anunciar el uso de cámara o micrófono: {error}");
    }
}

/// Los nodos de vídeo que existen ahora mismo.
fn nodos_de_video() -> Vec<PathBuf> {
    let Ok(entradas) = std::fs::read_dir("/dev") else {
        return Vec::new();
    };

    entradas
        .flatten()
        .map(|entrada| entrada.path())
        .filter(|ruta| ruta.to_str().is_some_and(es_nodo_de_video))
        .collect()
}

/// Sigue las aperturas y cierres de cada cámara.
///
/// Se vigila también el directorio `/dev`, porque una webcam USB enchufada a
/// mitad de sesión crea un nodo que no existía al arrancar. El del teléfono no
/// necesita esto —el módulo del loopback se carga en el arranque y el nodo está
/// desde siempre— pero una cámara de verdad sí.
async fn vigilar_las_camaras(app: AppHandle, ultimo: Ultimo) {
    use futures_util::StreamExt;
    use inotify::{Inotify, WatchMask};

    let inotify = match Inotify::init() {
        Ok(inotify) => inotify,
        Err(error) => {
            log::warn!("Sin inotify no hay aviso de cámara en uso: {error}");
            return;
        }
    };

    let mut watches = inotify.watches();

    // Nodos que aparecen y desaparecen.
    if let Err(error) = watches.add("/dev", WatchMask::CREATE | WatchMask::DELETE) {
        log::warn!(
            "No se pudo vigilar /dev, una cámara enchufada después pasará desapercibida: {error}"
        );
    }

    let mut vigilados: BTreeSet<PathBuf> = BTreeSet::new();
    let mut vigilar = |ruta: &PathBuf, vigilados: &mut BTreeSet<PathBuf>| {
        if vigilados.contains(ruta) {
            return;
        }
        match watches.add(ruta, WatchMask::OPEN | WatchMask::CLOSE) {
            Ok(_) => {
                vigilados.insert(ruta.clone());
            }
            Err(error) => log::warn!("No se pudo vigilar {}: {error}", ruta.display()),
        }
    };

    for nodo in nodos_de_video() {
        vigilar(&nodo, &mut vigilados);
    }

    // El estado al arrancar: puede haber algo grabando desde antes.
    let camara = quien_usa_la_camara();
    let microfono = ultimo.lock().await.microfono.clone();
    anunciar(&app, &ultimo, EnUso { camara, microfono }).await;

    let buffer = [0u8; 4096];
    let mut eventos = match inotify.into_event_stream(buffer) {
        Ok(eventos) => eventos,
        Err(error) => {
            log::warn!("No se pudo abrir el flujo de eventos de inotify: {error}");
            return;
        }
    };

    while let Some(evento) = eventos.next().await {
        let Ok(evento) = evento else { continue };

        // Un nodo nuevo hay que empezar a vigilarlo; si no, la cámara recién
        // enchufada queda muda para siempre.
        if let Some(nombre) = evento.name.as_ref().and_then(|n| n.to_str()) {
            let ruta = PathBuf::from("/dev").join(nombre);
            if ruta.to_str().is_some_and(es_nodo_de_video) {
                vigilar(&ruta, &mut vigilados);
            }
        }

        let camara = quien_usa_la_camara();
        let microfono = ultimo.lock().await.microfono.clone();
        anunciar(&app, &ultimo, EnUso { camara, microfono }).await;
    }
}

/// Sigue los flujos de captura de audio.
///
/// `pactl subscribe` es un proceso que no hace nada hasta que el audio cambia,
/// y ya se usa así para el micrófono silenciado. Recién cuando avisa de un
/// `source-output` se pide la foto del grafo, que es lo caro.
async fn vigilar_los_microfonos(app: AppHandle, ultimo: Ultimo) {
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    let camara = ultimo.lock().await.camara.clone();
    let microfono = mirar_los_microfonos().await;
    anunciar(&app, &ultimo, EnUso { camara, microfono }).await;

    let mut espera = std::time::Duration::from_secs(1);

    loop {
        let hijo = Command::new("pactl")
            .arg("subscribe")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        let mut hijo = match hijo {
            Ok(hijo) => hijo,
            Err(error) => {
                log::error!("No se pudo iniciar `pactl subscribe`: {error}");
                tokio::time::sleep(espera).await;
                // Sin esto, un PulseAudio ausente se vuelve un bucle cerrado.
                espera = (espera * 2).min(std::time::Duration::from_secs(30));
                continue;
            }
        };

        espera = std::time::Duration::from_secs(1);

        let Some(stdout) = hijo.stdout.take() else {
            let _ = hijo.kill().await;
            continue;
        };

        let mut lineas = BufReader::new(stdout).lines();

        // Cuando esto deja de dar líneas —se cayó el servidor de audio, o el
        // proceso murió— se sale a reconectar en vez de quedarse sordo.
        while let Ok(Some(linea)) = lineas.next_line().await {
            // Sólo los flujos de captura pueden cambiar quién escucha.
            if !linea.contains("source-output") {
                continue;
            }

            let camara = ultimo.lock().await.camara.clone();
            let microfono = mirar_los_microfonos().await;
            anunciar(&app, &ultimo, EnUso { camara, microfono }).await;
        }

        let _ = hijo.kill().await;
    }
}

#[async_trait]
impl Applet for PrivacidadApplet {
    fn name(&self) -> &'static str {
        "privacidad"
    }

    async fn start(&self, app: AppHandle) -> Result<(), Box<dyn Error>> {
        let ultimo: Ultimo = Arc::new(Mutex::new(EnUso::default()));

        {
            let app = app.clone();
            let ultimo = ultimo.clone();
            tokio::spawn(async move { vigilar_las_camaras(app, ultimo).await });
        }

        tokio::spawn(async move { vigilar_los_microfonos(app, ultimo).await });

        Ok(())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn reconoce_los_nodos_de_video_y_nada_mas() {
        assert!(es_nodo_de_video("/dev/video0"));
        assert!(es_nodo_de_video("/dev/video12"));
        assert!(!es_nodo_de_video("/dev/video"));
        assert!(!es_nodo_de_video("/dev/video0-metadata"));
        assert!(!es_nodo_de_video("/dev/videocaca"));
        assert!(!es_nodo_de_video("/home/pato/video0"));
    }

    #[test]
    fn distingue_al_que_escribe_del_que_mira() {
        // Medidos abriendo /dev/video0 de las dos formas.
        assert!(!solo_escribe("flags:\t0100000"), "lectura");
        assert!(solo_escribe("flags:\t0100001"), "escritura");
        // scrcpy abre en lectura y escritura: la bandera sola no lo excluye.
        assert!(!solo_escribe("flags:\t0100002"), "lectura y escritura");
        assert!(!solo_escribe("flags:"), "línea incompleta");
        assert!(!solo_escribe("flags:\tno-es-un-numero"));
    }

    #[test]
    fn saca_el_padre_aunque_el_nombre_tenga_parentesis() {
        assert_eq!(padre_de("42 (bash) S 7 42 42 0 -1").unwrap(), 7);
        // Un proceso puede llamarse así, y cortar por el primer `)` daría 0.
        assert_eq!(padre_de("42 (mi (app)) S 99 42 42").unwrap(), 99);
        assert_eq!(padre_de("sin parentesis"), None);
    }

    /// El caso que motivó seguir los enlaces: el flujo dice que graba un
    /// monitor, y el enlace real va al micrófono.
    #[test]
    fn el_enlace_manda_sobre_lo_que_dice_el_flujo() {
        let dump = serde_json::json!([
            {"id": 51, "type": "PipeWire:Interface:Node",
             "info": {"props": {"media.class": "Audio/Source", "node.description": "Micrófono"}}},
            {"id": 69, "type": "PipeWire:Interface:Node",
             "info": {"state": "running",
                      "props": {"media.class": "Stream/Input/Audio",
                                "application.name": "pw-record",
                                "target.object": "algo.monitor"}}},
            {"id": 72, "type": "PipeWire:Interface:Link",
             "info": {"input-node-id": 69, "output-node-id": 51}}
        ]);

        let usos = microfonos_en_uso(&dump);
        assert_eq!(usos.len(), 1);
        assert_eq!(usos[0].aplicacion, "pw-record");
        assert_eq!(usos[0].detalle, "Micrófono");
    }

    #[test]
    fn grabar_el_eco_del_altavoz_no_es_escuchar_el_microfono() {
        let dump = serde_json::json!([
            {"id": 46, "type": "PipeWire:Interface:Node",
             "info": {"props": {"media.class": "Audio/Sink", "node.description": "Altavoces"}}},
            {"id": 70, "type": "PipeWire:Interface:Node",
             "info": {"state": "running",
                      "props": {"media.class": "Stream/Input/Audio",
                                "application.name": "grabador-de-pantalla"}}},
            {"id": 73, "type": "PipeWire:Interface:Link",
             "info": {"input-node-id": 70, "output-node-id": 46}}
        ]);

        assert!(microfonos_en_uso(&dump).is_empty());
    }

    #[test]
    fn un_flujo_parado_no_escucha() {
        let dump = serde_json::json!([
            {"id": 51, "type": "PipeWire:Interface:Node",
             "info": {"props": {"media.class": "Audio/Source", "node.description": "Micrófono"}}},
            {"id": 69, "type": "PipeWire:Interface:Node",
             "info": {"state": "suspended",
                      "props": {"media.class": "Stream/Input/Audio", "application.name": "zoom"}}},
            {"id": 72, "type": "PipeWire:Interface:Link",
             "info": {"input-node-id": 69, "output-node-id": 51}}
        ]);

        assert!(microfonos_en_uso(&dump).is_empty());
    }

    #[test]
    fn un_dump_que_no_es_una_lista_no_rompe_nada() {
        assert!(microfonos_en_uso(&serde_json::json!({})).is_empty());
        assert!(microfonos_en_uso(&serde_json::json!([])).is_empty());
    }

    /// Dos aplicaciones grabando a la vez es el caso normal de una
    /// videollamada con un grabador encima, y las dos tienen que aparecer.
    #[test]
    fn lista_a_todos_los_que_escuchan() {
        let dump = serde_json::json!([
            {"id": 51, "type": "PipeWire:Interface:Node",
             "info": {"props": {"media.class": "Audio/Source", "node.description": "Micrófono"}}},
            {"id": 69, "type": "PipeWire:Interface:Node",
             "info": {"state": "running",
                      "props": {"media.class": "Stream/Input/Audio", "application.name": "Firefox"}}},
            {"id": 70, "type": "PipeWire:Interface:Node",
             "info": {"state": "running",
                      "props": {"media.class": "Stream/Input/Audio", "application.name": "OBS"}}},
            {"id": 72, "type": "PipeWire:Interface:Link",
             "info": {"input-node-id": 69, "output-node-id": 51}},
            {"id": 73, "type": "PipeWire:Interface:Link",
             "info": {"input-node-id": 70, "output-node-id": 51}}
        ]);

        let usos = microfonos_en_uso(&dump);
        assert_eq!(usos.len(), 2);
        assert!(usos.iter().any(|uso| uso.aplicacion == "Firefox"));
        assert!(usos.iter().any(|uso| uso.aplicacion == "OBS"));
    }

    /// La lista real de la máquina: no se puede predecir qué devuelve, pero sí
    /// que no explota y que no inventa a nadie sin cámara abierta.
    #[test]
    fn recorrer_proc_no_explota() {
        for uso in quien_usa_la_camara() {
            assert!(!uso.aplicacion.is_empty());
            assert!(uso.detalle.starts_with("/dev/video"));
        }
    }

    /// Con una cámara de verdad abierta, el recorrido tiene que encontrarla.
    ///
    /// Se abre el descriptor y se cierra sin leer un solo fotograma: alcanza
    /// para que el kernel registre la apertura y no se captura ninguna imagen.
    /// Sin cámaras en la máquina el test no tiene nada que comprobar y se va.
    #[test]
    fn encuentra_a_quien_tiene_la_camara_abierta() {
        let Some(nodo) = nodos_de_video().into_iter().next() else {
            return;
        };

        let Ok(abierto) = std::fs::File::open(&nodo) else {
            // Sin permiso sobre el grupo `video`, que es una configuración de
            // la máquina y no un error de esto.
            return;
        };

        let yo = comm_de(&PathBuf::from("/proc/self")).expect("sin comm propio");
        let usos = quien_usa_la_camara();
        drop(abierto);

        assert!(
            usos.iter().any(|uso| uso.aplicacion == yo),
            "la cámara estaba abierta por este proceso ({yo}) y no apareció: {usos:?}"
        );
    }
}
