//! Historial del portapapeles, estilo ChromeOS (Buscar+V) — vasak-desktop#217.
//!
//! v1: **sólo texto** y **sólo en memoria** (no toca el disco). Un vigía
//! (`watcher.rs`) escucha cada copia por data-control y la guarda en un anillo
//! acotado (`store.rs`); al elegir una entrada, el escritorio se hace dueño de la
//! selección y vuelve a servir ese texto (`source.rs`). La exclusión de secretos
//! vive en `store::should_skip`.
//!
//! La superposición que lo muestra es una superficie de capa aparte
//! (`windows_apps/clipboard.rs`), y el atajo llega por D-Bus (`dbus_service.rs`).

mod globals;
pub mod source;
pub mod store;
pub mod watcher;

pub use store::Entry;

/// Arranca el vigía del portapapeles. Se llama una vez al iniciar la sesión.
pub fn start() {
    watcher::start();
}

/// El historial actual, de lo más reciente a lo más viejo.
#[tauri::command]
pub fn clipboard_history() -> Vec<Entry> {
    store::snapshot()
}

/// Vuelve a poner la entrada `id` en el portapapeles.
///
/// Se la sube al frente del historial en el acto —así la vista queda al día sin
/// esperar a que el vigía vea nuestra propia selección, que de todos modos se
/// deduplica— y se la ofrece como selección.
#[tauri::command]
pub fn clipboard_pick(id: u64) -> Result<(), String> {
    match store::text_of(id) {
        Some(text) => {
            store::remember(text.clone());
            source::serve(text);
            Ok(())
        }
        None => Err(format!("no existe la entrada {id}")),
    }
}

/// Vacía el historial. No toca el portapapeles vivo, sólo lo recordado.
#[tauri::command]
pub fn clipboard_clear() {
    store::clear();
}

#[cfg(test)]
mod command_tests {
    use super::*;

    /// Un solo test: el historial es global, así que se ejerce en serie acá en vez
    /// de en varios tests que correrían en paralelo pisándose.
    #[test]
    fn leer_elegir_y_limpiar_el_historial() {
        store::clear();
        assert!(clipboard_history().is_empty());

        store::remember("alfa".into());
        store::remember("beta".into());
        let historial = clipboard_history();
        assert_eq!(historial.len(), 2);
        // La más reciente, al frente.
        assert_eq!(historial[0].text, "beta");

        let id_alfa = historial
            .iter()
            .find(|entry| entry.text == "alfa")
            .expect("alfa está")
            .id;
        // Elegir una entrada que existe la sube al frente y no falla. Servir el
        // portapapeles arranca un hilo aparte que, sin compositor, termina solo.
        assert!(clipboard_pick(id_alfa).is_ok());
        assert_eq!(clipboard_history()[0].text, "alfa");

        // Una entrada que no existe: error, sin tocar el historial.
        assert!(clipboard_pick(999_999).is_err());
        assert_eq!(clipboard_history().len(), 2);

        clipboard_clear();
        assert!(clipboard_history().is_empty());
    }
}

/// Contra el compositor de verdad, con la sesión andando. Se corre a mano
/// —`cargo test -- --ignored copiar_se_guarda_y_elegir_vuelve`— porque necesita
/// un Wayland que ofrezca data-control, que en una máquina de integración no
/// está. Es la verificación de la parte no rutinaria: el vigía y la fuente,
/// contra `wl-copy`/`wl-paste`, cada uno con su conexión y su hilo.
#[cfg(test)]
mod prueba_en_vivo {
    use super::*;
    use std::time::Duration;

    #[ignore = "necesita una sesión de Wayland con data-control (y wl-clipboard)"]
    #[test]
    fn copiar_se_guarda_y_elegir_vuelve_al_portapapeles() {
        watcher::start();
        std::thread::sleep(Duration::from_millis(500));

        // Otra app copia: tiene que aparecer en el historial.
        let copiado = format!("vasak-clip-copiado-{}", uuid::Uuid::new_v4());
        std::process::Command::new("wl-copy")
            .arg(&copiado)
            .status()
            .expect("wl-copy tiene que estar");
        std::thread::sleep(Duration::from_millis(600));
        assert!(
            store::snapshot().iter().any(|entry| entry.text == copiado),
            "el vigía no guardó lo que copió wl-copy"
        );

        // Elegir una entrada: el escritorio se hace dueño y la sirve.
        let elegido = format!("vasak-clip-elegido-{}", uuid::Uuid::new_v4());
        store::remember(elegido.clone());
        let id = store::snapshot()
            .iter()
            .find(|entry| entry.text == elegido)
            .expect("la entrada recién guardada")
            .id;
        clipboard_pick(id).expect("elegir no falla");
        std::thread::sleep(Duration::from_millis(600));

        // Otra app pega: tiene que ver lo elegido (prueba que la fuente sirve los
        // bytes y que las dos conexiones no se traban entre sí).
        let salida = std::process::Command::new("wl-paste")
            .arg("--no-newline")
            .output()
            .expect("wl-paste tiene que estar");
        assert_eq!(
            String::from_utf8_lossy(&salida.stdout),
            elegido,
            "wl-paste no devolvió lo que se eligió"
        );
    }
}
