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
