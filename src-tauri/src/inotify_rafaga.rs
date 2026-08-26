//! Esperar cambios en el disco sin despertar cuando no pasa nada.
//!
//! Los dos vigilantes del escritorio —el del menú y el de los archivos— tienen
//! el mismo problema: los cambios llegan en ráfaga (un gestor de paquetes
//! escribe cientos de archivos seguidos, una descarga crea y renombra) y actuar
//! por cada evento significa rehacer el mismo trabajo muchas veces. Pero
//! esperar a que la ráfaga se apague no debería costar despertares.

use inotify::Inotify;
use std::time::Duration;

/// Espera a que haya cambios y a que dejen de haberlos.
///
/// Bloquea en el kernel hasta el primer evento —mientras no pasa nada, el hilo
/// no despierta ni una vez— y después va drenando hasta que pasa `reposo` sin
/// novedades. Recién ahí vuelve, y el llamador hace su trabajo una sola vez.
///
/// La forma anterior era un `read_events` no bloqueante seguido de
/// `sleep(200ms)` en un bucle: correcto, pero **cinco despertares por segundo
/// para siempre**, unos 144 mil en una jornada de ocho horas, casi todos para
/// descubrir que no había nada. El costo de cada uno es chico; el de tenerlos
/// dos veces en el proceso que dibuja el escritorio, no tanto.
pub fn esperar_rafaga(
    inotify: &mut Inotify,
    buffer: &mut [u8],
    reposo: Duration,
) -> std::io::Result<()> {
    // `read_events_blocking` pone el descriptor en modo bloqueante y lo
    // restaura al volver, así que se puede alternar con `read_events` sin
    // tocar nada a mano.
    if inotify.read_events_blocking(buffer)?.count() == 0 {
        return Ok(());
    }

    loop {
        std::thread::sleep(reposo);

        match inotify.read_events(buffer) {
            Ok(eventos) => {
                // Sigue llegando: la ráfaga no terminó.
                if eventos.count() == 0 {
                    return Ok(());
                }
            }
            // Sin eventos pendientes es justamente la señal de que se apagó, no
            // un error.
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inotify::WatchMask;

    /// Un directorio propio para no pisar a otro test que corra a la par.
    fn directorio_de_prueba(nombre: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("vsk-rafaga-{}-{}", std::process::id(), nombre));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("crear el directorio de prueba");
        dir
    }

    /// Que despierte cuando algo cambia, y no antes.
    ///
    /// Las dos mitades importan: si no despertara, el widget de archivos no se
    /// actualizaría nunca —que es peor que releer cada diez segundos—, y si
    /// despertara sin cambios volveríamos al sondeo que este módulo vino a
    /// sacar.
    #[test]
    fn despierta_cuando_cambia_el_directorio_y_espera_a_que_la_rafaga_termine() {
        let dir = directorio_de_prueba("cambia");
        let mut inotify = Inotify::init().expect("inotify");
        inotify
            .watches()
            .add(&dir, WatchMask::CREATE | WatchMask::CLOSE_WRITE)
            .expect("vigilar");

        let reposo = Duration::from_millis(150);
        let escritor = dir.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            // Una ráfaga, como una descarga que se descomprime.
            for i in 0..5 {
                let _ = std::fs::write(escritor.join(format!("archivo-{i}")), b"x");
                std::thread::sleep(Duration::from_millis(30));
            }
        });

        let inicio = std::time::Instant::now();
        let mut buffer = [0u8; 4096];
        esperar_rafaga(&mut inotify, &mut buffer, reposo).expect("esperar");
        let tardanza = inicio.elapsed();

        assert!(
            tardanza >= Duration::from_millis(200),
            "no puede volver antes de que haya un cambio; volvió en {tardanza:?}"
        );
        assert!(
            tardanza >= Duration::from_millis(200) + reposo,
            "tiene que esperar a que la ráfaga se apague antes de avisar; volvió en {tardanza:?}"
        );

        // Y una sola vez para las cinco escrituras: eso es el punto del reposo.
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Sin cambios no vuelve, que es lo que ahorra los despertares. Se comprueba
    /// por el negativo: en medio segundo largo sin tocar nada, no volvió.
    #[test]
    fn sin_cambios_no_vuelve() {
        let dir = directorio_de_prueba("quieto");
        let mut inotify = Inotify::init().expect("inotify");
        inotify.watches().add(&dir, WatchMask::CREATE).expect("vigilar");

        let (aviso, recibo) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            let _ = esperar_rafaga(&mut inotify, &mut buffer, Duration::from_millis(50));
            let _ = aviso.send(());
        });

        assert!(
            recibo.recv_timeout(Duration::from_millis(600)).is_err(),
            "volvió sin que nada cambiara: eso es sondear, no escuchar"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
