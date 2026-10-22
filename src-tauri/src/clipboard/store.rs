//! El historial en memoria y su política, aislada en funciones puras.
//!
//! Todo lo que decide qué entra y en qué orden —descartar secretos, elegir el
//! MIME de texto, deduplicar y acotar— vive en funciones puras, para que se
//! prueben sin un compositor de Wayland delante. El anillo en sí es un
//! [`VecDeque`] detrás de un mutex, global al proceso: lo llena el vigía
//! (`watcher.rs`) y lo leen los comandos. **Nunca toca el disco** (vasak-desktop#217,
//! v1): al cerrar la sesión el proceso muere y con él el historial.

use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex};

use serde::Serialize;

/// Cuántas entradas se guardan (vasak-desktop#217, v1).
pub const CAPACITY: usize = 25;

/// No se guarda un pegado más grande que esto: un historial en memoria no es
/// lugar para un archivo entero pegado como texto, y leer sin tope deja que
/// cualquier app llene la RAM por el portapapeles.
pub const MAX_BYTES: u64 = 1024 * 1024;

/// El MIME centinela con que un gestor de contraseñas marca lo sensible.
///
/// KeePassXC y compañía ofrecen este tipo junto al texto para pedir, por
/// convención del ecosistema, que un gestor de portapapeles no lo guarde. Es el
/// único centinela por omisión; la lista es ampliable (ver [`should_skip`]).
pub const SECRET_HINT: &str = "x-kde-passwordManagerHint";

/// Los MIME de texto que sabemos leer, del más preciso al más tosco.
pub const TEXT_MIMES: [&str; 4] = [
    "text/plain;charset=utf-8",
    "text/plain",
    "UTF8_STRING",
    "STRING",
];

/// Una entrada del historial: el texto y un id estable mientras viva.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Entry {
    pub id: u64,
    pub text: String,
}

/// Si la oferta hay que descartarla entera **antes** de guardarla.
///
/// Se descarta cuando un centinela la marca como secreta —el límite honesto: una
/// contraseña copiada a mano, sin ese centinela, no se distingue de cualquier
/// texto—, o cuando no ofrece ningún tipo de texto que sepamos leer (una imagen,
/// o sólo tipos de pista): no hay nada que guardar.
pub fn should_skip(mimes: &[String], denylist: &[String]) -> bool {
    is_secret(mimes, denylist) || pick_text_mime(mimes).is_none()
}

fn is_secret(mimes: &[String], denylist: &[String]) -> bool {
    mimes
        .iter()
        .any(|mime| mime.contains(SECRET_HINT) || denylist.iter().any(|deny| mime == deny))
}

/// El mejor MIME de texto ofrecido para leer, si hay alguno.
pub fn pick_text_mime(mimes: &[String]) -> Option<&'static str> {
    TEXT_MIMES
        .into_iter()
        .find(|wanted| mimes.iter().any(|mime| mime == wanted))
}

/// Mete `text` al frente del anillo, deduplicando y acotando a `cap`.
///
/// - Vacío: nada.
/// - Ya es la cabeza: nada (así el vigía no reacciona a nuestra propia
///   selección cuando se elige una entrada; ya quedó arriba).
/// - Ya estaba más abajo: se mueve al frente conservando su id.
/// - Nuevo: id nuevo, al frente, y se recorta la cola.
///
/// Devuelve si el anillo cambió.
pub fn insert(ring: &mut VecDeque<Entry>, text: String, cap: usize, next_id: &mut u64) -> bool {
    if text.is_empty() {
        return false;
    }
    if let Some(position) = ring.iter().position(|entry| entry.text == text) {
        if position == 0 {
            return false;
        }
        let entry = ring
            .remove(position)
            .expect("la posición acaba de encontrarse");
        ring.push_front(entry);
        return true;
    }
    *next_id += 1;
    ring.push_front(Entry { id: *next_id, text });
    while ring.len() > cap {
        ring.pop_back();
    }
    true
}

// ── El anillo global ───────────────────────────────────────────────────────────

struct Store {
    ring: VecDeque<Entry>,
    next_id: u64,
}

static STORE: LazyLock<Mutex<Store>> = LazyLock::new(|| {
    Mutex::new(Store {
        ring: VecDeque::new(),
        next_id: 0,
    })
});

fn lock() -> std::sync::MutexGuard<'static, Store> {
    // Un pánico con el mutex tomado no deja el historial en un estado a medias:
    // es un `VecDeque` y un contador. Mejor seguir que arrastrar el veneno.
    STORE.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// Guarda `text` al frente del historial (dedup + tope). Lo llama el vigía.
pub fn remember(text: String) {
    let mut guard = lock();
    let store = &mut *guard;
    insert(&mut store.ring, text, CAPACITY, &mut store.next_id);
}

/// Una copia del historial, de la más reciente a la más vieja.
pub fn snapshot() -> Vec<Entry> {
    lock().ring.iter().cloned().collect()
}

/// El texto de la entrada `id`, si sigue en el historial.
pub fn text_of(id: u64) -> Option<String> {
    lock()
        .ring
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.text.clone())
}

/// Vacía el historial. No toca el portapapeles vivo, sólo lo recordado.
pub fn clear() {
    lock().ring.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mimes(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn se_descarta_lo_que_marca_un_centinela_de_secreto() {
        let secreto = mimes(&["text/plain;charset=utf-8", SECRET_HINT]);
        assert!(should_skip(&secreto, &[]));
        // El centinela a veces llega con sufijo (el valor «secret»): basta que
        // el nombre del tipo lo contenga.
        let con_sufijo = mimes(&["text/plain", "x-kde-passwordManagerHint.secret"]);
        assert!(should_skip(&con_sufijo, &[]));
    }

    #[test]
    fn se_descarta_una_lista_configurable() {
        let denylist = mimes(&["application/x-super-secret"]);
        let oferta = mimes(&["text/plain", "application/x-super-secret"]);
        assert!(should_skip(&oferta, &denylist));
        // Sin la lista, el mismo tipo no basta para descartar.
        assert!(!should_skip(&oferta, &[]));
    }

    #[test]
    fn se_descarta_lo_que_no_trae_texto_que_sepamos_leer() {
        assert!(should_skip(&mimes(&["image/png"]), &[]));
        // Sólo tipos de pista, sin texto: nada que guardar.
        assert!(should_skip(&mimes(&["x-kde-passwordManagerHint"]), &[]));
        assert!(!should_skip(&mimes(&["text/plain;charset=utf-8"]), &[]));
    }

    #[test]
    fn se_elige_el_mime_de_texto_mas_preciso() {
        assert_eq!(
            pick_text_mime(&mimes(&[
                "STRING",
                "text/plain",
                "text/plain;charset=utf-8"
            ])),
            Some("text/plain;charset=utf-8")
        );
        assert_eq!(
            pick_text_mime(&mimes(&["STRING", "UTF8_STRING"])),
            Some("UTF8_STRING")
        );
        assert_eq!(pick_text_mime(&mimes(&["image/png"])), None);
    }

    #[test]
    fn insertar_deduplica_moviendo_al_frente_y_conserva_el_id() {
        let mut ring = VecDeque::new();
        let mut id = 0;
        assert!(insert(&mut ring, "uno".into(), 25, &mut id));
        assert!(insert(&mut ring, "dos".into(), 25, &mut id));
        let id_de_uno = ring.iter().find(|e| e.text == "uno").unwrap().id;

        // «uno» otra vez: sube al frente con su id, sin crear otra entrada.
        assert!(insert(&mut ring, "uno".into(), 25, &mut id));
        assert_eq!(ring.len(), 2);
        assert_eq!(ring.front().unwrap().text, "uno");
        assert_eq!(ring.front().unwrap().id, id_de_uno);
    }

    #[test]
    fn insertar_lo_que_ya_es_cabeza_no_cambia_nada() {
        let mut ring = VecDeque::new();
        let mut id = 0;
        insert(&mut ring, "cabeza".into(), 25, &mut id);
        // Es lo que pasa cuando se elige una entrada: el vigía ve nuestra propia
        // selección y no tiene que hacer nada.
        assert!(!insert(&mut ring, "cabeza".into(), 25, &mut id));
        assert_eq!(ring.len(), 1);
    }

    #[test]
    fn insertar_vacio_no_guarda_nada() {
        let mut ring = VecDeque::new();
        let mut id = 0;
        assert!(!insert(&mut ring, String::new(), 25, &mut id));
        assert!(ring.is_empty());
    }

    #[test]
    fn el_anillo_no_pasa_del_tope() {
        let mut ring = VecDeque::new();
        let mut id = 0;
        for n in 0..30 {
            insert(&mut ring, format!("texto {n}"), CAPACITY, &mut id);
        }
        assert_eq!(ring.len(), CAPACITY);
        // La más nueva arriba, la vieja descartada.
        assert_eq!(ring.front().unwrap().text, "texto 29");
        assert!(!ring.iter().any(|e| e.text == "texto 0"));
    }
}
