//! Los mapas de bits que mandan los elementos de la bandeja, convertidos a PNG
//! y con tope de tamaño.
//!
//! Llegan de dos lados y en dos formatos:
//!
//! - **StatusNotifierItem** (`IconPixmap`, `OverlayIconPixmap`,
//!   `AttentionIconPixmap` y el icono del `ToolTip`): `a(iiay)`, varios tamaños
//!   del mismo icono, cada uno en ARGB32 «en el orden de bytes de la red», o
//!   sea `[A, R, G, B]` por píxel.
//! - **dbusmenu** (`icon-data`): los bytes de un PNG.
//!
//! Los dos vienen de otro proceso, así que no se les cree nada: un ancho
//! negativo, un largo que no cierra con el ancho por el alto, un PNG roto o uno
//! de diez mil píxeles de lado no llegan a la vista. La vista recibe un PNG en
//! base64 o nada.

use base64::{engine::general_purpose, Engine as _};

/// Un icono de la bandeja se dibuja a 16 px lógicos; con escala, 32 o 48. Lo que
/// pase de esto se achica antes de mandarlo, para no pasarle a la vista un
/// base64 de un megabyte por un icono de 16 px.
pub const MAX_ICON_SIDE: u32 = 128;

/// El lado más grande que se acepta leer de un `IconPixmap`. Más que esto no es
/// un icono —un 1024×1024 son 4 MiB por cada señal `NewIcon`— y se descarta sin
/// convertirlo.
pub const MAX_PIXMAP_SIDE: i32 = 512;

/// Lo más que se acepta de un `icon-data` de dbusmenu. Un icono de menú de 16 px
/// en PNG pesa menos de 2 KiB (el de Discord, medido, 905 bytes).
pub const MAX_MENU_ICON_BYTES: usize = 256 * 1024;

/// La firma de un PNG.
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

pub type Pixmap = (i32, i32, Vec<u8>);

fn is_valid_pixmap(pixmap: &Pixmap) -> bool {
    let (width, height, data) = pixmap;
    *width > 0
        && *height > 0
        && *width <= MAX_PIXMAP_SIDE
        && *height <= MAX_PIXMAP_SIDE
        && data.len() == (*width as usize) * (*height as usize) * 4
}

/// Las aplicaciones publican el mismo icono en varios tamaños. El panel lo
/// dibuja a 16 px lógicos, que en pantallas con escala son 32 o más, así que
/// tomar el primero (normalmente 16×16) daba un icono borroso al ampliarlo. Se
/// elige el más chico que llegue a 48 px, y si ninguno llega, el mayor. Los que
/// no cierran o pasan del tope no se consideran.
pub fn pick_pixmap(pixmaps: &[Pixmap]) -> Option<&Pixmap> {
    pixmaps
        .iter()
        .filter(|p| is_valid_pixmap(p))
        .filter(|p| p.0.min(p.1) >= 48)
        .min_by_key(|p| p.0.min(p.1))
        .or_else(|| {
            pixmaps
                .iter()
                .filter(|p| is_valid_pixmap(p))
                .max_by_key(|p| p.0.min(p.1))
        })
}

fn encode_png(img: image::RgbaImage) -> Option<String> {
    let longest = img.width().max(img.height());
    let img = if longest > MAX_ICON_SIDE {
        // Se achica respetando la proporción: un icono apaisado sigue apaisado.
        let scale =
            |side: u32| ((side as u64 * MAX_ICON_SIDE as u64) / longest as u64).max(1) as u32;
        image::imageops::thumbnail(&img, scale(img.width()), scale(img.height()))
    } else {
        img
    };
    let mut buffer = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buffer),
        image::ImageFormat::Png,
    )
    .ok()?;
    Some(general_purpose::STANDARD.encode(&buffer))
}

/// Un `IconPixmap` (ARGB32, orden de red) a PNG en base64.
///
/// Antes se reordenaba como `[G, R, A, B]`, que rotaba los canales: el azul de
/// Telegram salía violeta y su insignia roja quedaba como un halo
/// semitransparente.
pub fn pixmap_to_png_base64(pixmap: &Pixmap) -> Option<String> {
    if !is_valid_pixmap(pixmap) {
        return None;
    }
    let (width, height, data) = pixmap;
    let mut rgba = Vec::with_capacity(data.len());
    for chunk in data.chunks_exact(4) {
        rgba.extend_from_slice(&[chunk[1], chunk[2], chunk[3], chunk[0]]);
    }
    // Un mapa de bits todo transparente es lo que manda Chromium cuando no
    // tiene icono que poner: dibujarlo es dibujar un hueco.
    if rgba.chunks_exact(4).all(|px| px[3] == 0) {
        return None;
    }
    let img = image::RgbaImage::from_raw(*width as u32, *height as u32, rgba)?;
    encode_png(img)
}

/// El mejor de varios tamaños, convertido. `None` si ninguno sirve.
pub fn best_pixmap_png_base64(pixmaps: &[Pixmap]) -> Option<String> {
    pick_pixmap(pixmaps).and_then(pixmap_to_png_base64)
}

/// El `icon-data` de una entrada de dbusmenu, validado.
///
/// Tiene que ser un PNG de verdad —la firma y la cabecera se leen— y no pasar
/// del tope. Si es más grande que [`MAX_ICON_SIDE`] se decodifica y se achica;
/// si no, se reenvía tal cual, sin volver a codificarlo.
pub fn menu_icon_png_base64(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() || bytes.len() > MAX_MENU_ICON_BYTES || !bytes.starts_with(PNG_SIGNATURE) {
        return None;
    }
    let reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let (width, height) = reader.into_dimensions().ok()?;
    if width == 0
        || height == 0
        || width > MAX_PIXMAP_SIDE as u32
        || height > MAX_PIXMAP_SIDE as u32
    {
        return None;
    }
    if width <= MAX_ICON_SIDE && height <= MAX_ICON_SIDE {
        // Que la cabecera esté bien no dice que el resto también: se decodifica
        // igual, para no mandarle a la vista un PNG que se dibuja roto.
        image::load_from_memory_with_format(bytes, image::ImageFormat::Png).ok()?;
        return Some(general_purpose::STANDARD.encode(bytes));
    }
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .ok()?
        .to_rgba8();
    encode_png(img)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Un mapa de bits de un solo color, en ARGB.
    pub fn solid(width: i32, height: i32, argb: [u8; 4]) -> Pixmap {
        let data = argb.repeat((width * height) as usize);
        (width, height, data)
    }

    pub fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 255]));
        let mut buffer = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut buffer),
            image::ImageFormat::Png,
        )
        .unwrap();
        buffer
    }

    fn decode(b64: &str) -> image::RgbaImage {
        let bytes = general_purpose::STANDARD.decode(b64).unwrap();
        image::load_from_memory(&bytes).unwrap().to_rgba8()
    }

    #[test]
    fn el_argb_de_la_red_queda_en_su_lugar() {
        let png = pixmap_to_png_base64(&solid(2, 2, [255, 10, 20, 30])).unwrap();
        assert_eq!(decode(&png).get_pixel(0, 0).0, [10, 20, 30, 255]);
    }

    #[test]
    fn se_elige_el_menor_que_llega_a_48_y_si_no_el_mayor() {
        let set = vec![
            solid(16, 16, [255; 4]),
            solid(64, 64, [255; 4]),
            solid(48, 48, [255; 4]),
        ];
        assert_eq!(pick_pixmap(&set).unwrap().0, 48);
        let small = vec![solid(16, 16, [255; 4]), solid(22, 22, [255; 4])];
        assert_eq!(pick_pixmap(&small).unwrap().0, 22);
    }

    #[test]
    fn un_mapa_que_no_cierra_no_se_dibuja() {
        assert!(pixmap_to_png_base64(&(4, 4, vec![0; 10])).is_none());
        assert!(pixmap_to_png_base64(&(-1, 4, vec![])).is_none());
        assert!(pixmap_to_png_base64(&(0, 0, vec![])).is_none());
        assert!(best_pixmap_png_base64(&[]).is_none());
    }

    #[test]
    fn un_mapa_todo_transparente_es_un_hueco_y_no_se_manda() {
        assert!(pixmap_to_png_base64(&solid(8, 8, [0, 255, 255, 255])).is_none());
    }

    #[test]
    fn un_mapa_enorme_se_descarta_sin_convertirlo() {
        let huge = solid(MAX_PIXMAP_SIDE + 1, 4, [255; 4]);
        assert!(pick_pixmap(std::slice::from_ref(&huge)).is_none());
        // Con uno chico al lado, gana el chico.
        let set = vec![huge, solid(32, 32, [255; 4])];
        assert_eq!(pick_pixmap(&set).unwrap().0, 32);
    }

    #[test]
    fn uno_grande_dentro_del_tope_se_achica() {
        let png = pixmap_to_png_base64(&solid(256, 256, [255, 1, 2, 3])).unwrap();
        let img = decode(&png);
        assert_eq!((img.width(), img.height()), (MAX_ICON_SIDE, MAX_ICON_SIDE));
    }

    #[test]
    fn el_png_del_menu_se_reenvia_tal_cual() {
        let bytes = png_bytes(16, 16);
        let b64 = menu_icon_png_base64(&bytes).unwrap();
        assert_eq!(general_purpose::STANDARD.decode(b64).unwrap(), bytes);
    }

    #[test]
    fn un_png_invalido_no_llega_a_la_vista() {
        assert!(menu_icon_png_base64(&[]).is_none());
        assert!(menu_icon_png_base64(b"no soy un png").is_none());
        // La firma bien y el resto roto.
        let mut broken = PNG_SIGNATURE.to_vec();
        broken.extend_from_slice(&[0, 0, 0, 13, b'I', b'H', b'D', b'R', 1, 2]);
        assert!(menu_icon_png_base64(&broken).is_none());
        // La cabecera bien y los datos cortados.
        let mut cut = png_bytes(16, 16);
        cut.truncate(cut.len() - 20);
        assert!(menu_icon_png_base64(&cut).is_none());
    }

    #[test]
    fn un_png_del_menu_enorme_se_descarta_o_se_achica() {
        // Más bytes que el tope: ni se mira.
        let mut heavy = png_bytes(16, 16);
        heavy.resize(MAX_MENU_ICON_BYTES + 1, 0);
        assert!(menu_icon_png_base64(&heavy).is_none());
        // Más lado que el tope de lectura.
        assert!(menu_icon_png_base64(&png_bytes(MAX_PIXMAP_SIDE as u32 + 1, 1)).is_none());
        // Dentro del tope pero grande: se achica.
        let b64 = menu_icon_png_base64(&png_bytes(300, 300)).unwrap();
        assert_eq!(decode(&b64).width(), MAX_ICON_SIDE);
    }
}
