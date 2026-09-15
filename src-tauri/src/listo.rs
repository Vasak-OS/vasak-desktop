//! Avisarle a systemd que el escritorio ya está dibujado.
//!
//! # Por qué hace falta
//!
//! El escritorio es una unidad `Type=notify` ordenada delante de
//! `xdg-desktop-autostart.target`: mientras no diga que está listo, el inicio
//! automático no arranca. Sin eso, todo lo que abre al iniciar sesión —Claude,
//! Twingate, Steam en algunos equipos— sale un segundo después del compositor y
//! le pelea la máquina al escritorio justo mientras inicializa GTK y WebKit.
//!
//! Lo que costaba, medido: el mismo binario tarda **467 ms** con el sistema
//! tranquilo y **16.700 ms** al iniciar sesión.
//!
//! # Sin dependencia nueva
//!
//! El protocolo de `sd_notify` es escribir `READY=1` en un socket de datagramas
//! de UNIX cuya dirección está en `$NOTIFY_SOCKET`. Son quince líneas; sumar un
//! crate para esto sería más superficie que código.
//!
//! Un nombre que empieza con `@` es un socket abstracto, y ésos **no se pueden
//! nombrar como ruta**: la API de rutas de Rust rechaza cualquier cadena con un
//! byte cero adentro, así que reemplazar el `@` por `\0` y llamar a `send_to`
//! falla — y falla callado, porque el error se descarta. Van por
//! `SocketAddr::from_abstract_name` y `send_to_addr`, que es otra llamada.
//!
//! systemd usa socket abstracto salvo que se lo configure de otra forma, así que
//! ése es el camino normal y no el raro.
//!
//! # Todo falla en silencio, a propósito
//!
//! Corriendo a mano, fuera de systemd, no hay `$NOTIFY_SOCKET` y no hay nada que
//! avisar. Eso no es un error: es el caso de desarrollo. Y si el aviso falla
//! estando la variable, tampoco se puede hacer nada útil desde acá; lo peor que
//! pasa es que systemd espere hasta su propio tope.

use std::os::unix::net::UnixDatagram;

/// Le dice a systemd que el escritorio ya se puede usar.
///
/// Devuelve si el aviso salió. `false` incluye el caso normal de no estar
/// corriendo bajo systemd.
pub fn avisar_que_esta_listo() -> bool {
    match hay_a_quien_avisar(std::env::var("NOTIFY_SOCKET").ok().as_deref()) {
        Some(direccion) => avisar_a(direccion),
        None => false,
    }
}

/// A quién avisarle, si es que hay alguien.
///
/// Se separa de la lectura del entorno para poder probarla: `set_var` es global
/// al proceso y los tests corren en hilos, así que un test que manosee
/// `NOTIFY_SOCKET` se pisa con sus vecinos. Pasó — tres tests en paralelo se
/// robaban la variable entre sí y el cuarto fallaba sin motivo aparente.
fn hay_a_quien_avisar(valor: Option<&str>) -> Option<&str> {
    match valor {
        Some(v) if !v.is_empty() => Some(v),
        _ => None,
    }
}

/// Manda el `READY=1` a una dirección concreta.
fn avisar_a(direccion: &str) -> bool {
    let Ok(socket) = UnixDatagram::unbound() else {
        return false;
    };

    // `@` al principio marca un socket abstracto, que se nombra aparte de las
    // rutas. systemd usa ése salvo que se lo configure de otra forma.
    match direccion.strip_prefix('@') {
        Some(nombre) => {
            use std::os::linux::net::SocketAddrExt;
            let Ok(destino) = std::os::unix::net::SocketAddr::from_abstract_name(nombre.as_bytes())
            else {
                return false;
            };
            socket.send_to_addr(b"READY=1\n", &destino).is_ok()
        }
        None => socket.send_to(b"READY=1\n", direccion).is_ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sin la variable no hay a quién avisarle, y eso **no** es un fallo: es lo
    /// que pasa cuando alguien corre el escritorio a mano para probarlo.
    #[test]
    fn sin_la_variable_no_hay_a_quien_avisar() {
        assert_eq!(hay_a_quien_avisar(None), None);
    }

    /// Una variable vacía es lo mismo que no tenerla. systemd no la deja así,
    /// pero un envoltorio que la exporte mal sí.
    #[test]
    fn la_variable_vacia_cuenta_como_ausente() {
        assert_eq!(hay_a_quien_avisar(Some("")), None);
    }

    #[test]
    fn con_direccion_hay_a_quien_avisar() {
        assert_eq!(hay_a_quien_avisar(Some("/run/x")), Some("/run/x"));
    }

    /// El camino bueno, contra un socket de verdad: se comprueba que llegue
    /// exactamente `READY=1`, que es lo que systemd espera leer.
    #[test]
    fn avisa_por_el_socket_y_manda_lo_que_systemd_espera() {
        let dir = std::env::temp_dir().join(format!("vsk-listo-{}", std::process::id()));
        let _ = std::fs::remove_file(&dir);
        let receptor = UnixDatagram::bind(&dir).expect("no se pudo abrir el socket de prueba");

        assert!(avisar_a(dir.to_str().unwrap()));

        let mut buzon = [0u8; 64];
        let leidos = receptor.recv(&mut buzon).expect("no llegó nada");
        assert_eq!(&buzon[..leidos], b"READY=1\n");
        let _ = std::fs::remove_file(&dir);
    }

    /// Los sockets abstractos se nombran con `@` y **no se pueden pasar como
    /// ruta**: la API de rutas rechaza cualquier cadena con un byte cero, así
    /// que la versión que reemplazaba el `@` por `\0` y llamaba a `send_to`
    /// fallaba — y fallaba callada, porque el error se descarta. systemd usa
    /// socket abstracto por omisión, o sea que ese fallo silencioso era el caso
    /// normal: la unidad se habría colgado hasta su tope en cada arranque.
    #[test]
    fn el_socket_abstracto_recibe_el_aviso() {
        use std::os::linux::net::SocketAddrExt;
        let nombre = format!("vsk-listo-abstracto-{}", std::process::id());
        let direccion = std::os::unix::net::SocketAddr::from_abstract_name(nombre.as_bytes())
            .expect("nombre abstracto inválido");
        let receptor =
            UnixDatagram::bind_addr(&direccion).expect("no se pudo abrir el socket abstracto");

        assert!(avisar_a(&format!("@{nombre}")));

        let mut buzon = [0u8; 64];
        let leidos = receptor.recv(&mut buzon).expect("no llegó nada");
        assert_eq!(&buzon[..leidos], b"READY=1\n");
    }

    /// Una dirección que no existe no puede hacer caer al escritorio.
    #[test]
    fn una_direccion_que_no_existe_no_revienta() {
        assert!(!avisar_a("/no/existe/este/socket"));
    }
}
