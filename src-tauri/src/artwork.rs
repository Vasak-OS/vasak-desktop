//! La carátula que publica el reproductor, en bytes y por el IPC.
//!
//! El `mpris:artUrl` que llega del bus apunta casi siempre a un archivo, y cada
//! reproductor lo deja en un lugar distinto: Firefox en
//! `~/.mozilla/firefox/firefox-mpris/`, Chromium en `/tmp`, los de escritorio en
//! su propio cache. El WebView no puede leer ninguno directamente —el protocolo
//! de assets sirve sólo lo que esté dentro del alcance declarado, y contesta 403
//! a todo lo demás—, así que la carátula de Chromium **no se veía**: el `@error`
//! del `<img>` caía al icono genérico y parecía que el reproductor no mandaba
//! ninguna.
//!
//! La salida no es agrandar el alcance hasta que entre `/tmp`, que es de todos y
//! cuyos nombres los elige otro proceso. Es que la ruta no la abra el WebView:
//! llega acá, se comprueba que lo que hay sea de verdad una imagen, y lo que
//! vuelve son bytes que el frontend convierte en un `blob:` suyo.
//!
//! Lo remoto —Spotify publica `https://i.scdn.co/…`— no pasa por acá: lo carga
//! el WebView, que ya trae TLS, por el mismo motivo por el que el clima se pide
//! desde ahí y no desde Rust.

use std::path::PathBuf;

/// Lo más grande que se lee. Una carátula es una imagen de unos cientos de kB;
/// el tope está para que un `artUrl` apuntando a algo enorme no se lleve la
/// memoria del escritorio, no para recortar carátulas de verdad.
pub const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// La ruta local que hay detrás de un `mpris:artUrl`, si es que hay alguna.
///
/// Devuelve `None` para lo remoto y para `data:`, que no se leen del disco, y
/// para cualquier cosa que no termine en una ruta absoluta. El porcentaje se
/// decodifica porque la URL lo trae codificado y el sistema de archivos no:
/// `file:///tmp/a%20b.png` es el archivo `/tmp/a b.png`.
pub fn local_path(url: &str) -> Option<PathBuf> {
    let url = url.trim();
    let rest = if let Some(r) = url.strip_prefix("file://") {
        // `file://localhost/tmp/x` y `file:///tmp/x` son el mismo archivo.
        r.strip_prefix("localhost").unwrap_or(r)
    } else if url.starts_with('/') {
        url
    } else {
        return None;
    };

    let decoded = percent_encoding::percent_decode_str(rest)
        .decode_utf8()
        .ok()?;

    let path = PathBuf::from(decoded.as_ref());
    if path.is_absolute() {
        Some(path)
    } else {
        None
    }
}

/// Si lo que se leyó es de verdad una imagen.
///
/// Que la ruta la elija otro proceso es justamente el motivo: sin esto, un
/// `artUrl` apuntando a cualquier archivo del usuario haría que el escritorio lo
/// leyera y se lo entregara a su propio WebView. Mirar la cabecera no cuesta
/// nada y deja pasar sólo lo que el `<img>` iba a poder dibujar igual.
pub fn looks_like_image(bytes: &[u8]) -> bool {
    image::guess_format(bytes).is_ok()
}

/// Los bytes de la carátula, con el error diciendo **cuál** archivo falló.
pub async fn read_local(url: &str) -> Result<Vec<u8>, String> {
    let path = local_path(url).ok_or_else(|| format!("no es una ruta local: {url}"))?;

    let meta = tokio::fs::metadata(&path)
        .await
        .map_err(|e| format!("no se pudo mirar la carátula en {}: {e}", path.display()))?;
    if meta.len() > MAX_BYTES {
        return Err(format!(
            "la carátula en {} pesa {} bytes, más del tope de {MAX_BYTES}",
            path.display(),
            meta.len()
        ));
    }

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("no se pudo leer la carátula en {}: {e}", path.display()))?;

    if !looks_like_image(&bytes) {
        return Err(format!("lo que hay en {} no es una imagen", path.display()));
    }

    Ok(bytes)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// Los ocho bytes con los que empieza todo PNG, y algo de relleno.
    fn png() -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x0d, b'I', b'H', b'D', b'R']);
        bytes
    }

    #[test]
    fn la_ruta_sale_de_las_tres_formas_que_llegan() {
        // Chromium, que es el caso que estaba roto.
        assert_eq!(
            local_path("file:///tmp/.com.google.Chrome.9825ka"),
            Some(PathBuf::from("/tmp/.com.google.Chrome.9825ka"))
        );
        // Firefox, con la forma que lleva el host adentro.
        assert_eq!(
            local_path("file://localhost/home/quien/.mozilla/a.png"),
            Some(PathBuf::from("/home/quien/.mozilla/a.png"))
        );
        // Y una ruta pelada, que algunos reproductores mandan así.
        assert_eq!(
            local_path("/var/cache/tapa.jpg"),
            Some(PathBuf::from("/var/cache/tapa.jpg"))
        );
    }

    #[test]
    fn el_porcentaje_se_decodifica_porque_el_disco_no_lo_entiende() {
        assert_eq!(
            local_path("file:///tmp/tapa%20del%20disco.png"),
            Some(PathBuf::from("/tmp/tapa del disco.png"))
        );
    }

    #[test]
    fn lo_que_no_es_del_disco_no_devuelve_ruta() {
        for url in [
            "https://i.scdn.co/image/ab67616d",
            "http://ejemplo.test/tapa.png",
            "data:image/png;base64,iVBORw0KGgo=",
            "tapa.png",
            "",
        ] {
            assert_eq!(local_path(url), None, "{url} no es una ruta local");
        }
    }

    #[test]
    fn la_cabecera_distingue_una_imagen_de_lo_que_no_lo_es() {
        assert!(looks_like_image(&png()));
        assert!(!looks_like_image(b"-----BEGIN OPENSSH PRIVATE KEY-----"));
        assert!(!looks_like_image(b""));
    }

    #[tokio::test]
    async fn los_bytes_del_archivo_llegan_tal_cual() {
        let dir = std::env::temp_dir().join(format!("vsk-artwork-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tapa.png");
        std::fs::write(&path, png()).unwrap();

        let read = read_local(&format!("file://{}", path.display()))
            .await
            .expect("la carátula se tiene que poder leer");

        assert_eq!(read, png());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn lo_que_no_es_una_imagen_no_vuelve_aunque_exista() {
        let dir = std::env::temp_dir().join(format!("vsk-artwork-no-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("secreto");
        std::fs::write(&path, b"ssh-rsa AAAA...").unwrap();

        let error = read_local(&format!("file://{}", path.display()))
            .await
            .expect_err("un archivo que no es imagen no se entrega");

        assert!(error.contains("no es una imagen"), "{error}");
        assert!(
            error.contains("secreto"),
            "el error nombra el archivo: {error}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn el_error_de_lectura_nombra_la_ruta() {
        let error = read_local("file:///tmp/esta-caratula-no-existe-nunca.png")
            .await
            .expect_err("no está");
        assert!(
            error.contains("esta-caratula-no-existe-nunca.png"),
            "{error}"
        );
    }
}
