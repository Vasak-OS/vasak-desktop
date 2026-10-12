use gtk::prelude::*;
use tauri::{AppHandle, Url, WebviewUrl, WebviewWindowBuilder};
use url::form_urlencoded::Serializer;

use crate::app_url::get_app_url;
use crate::gtk_utils;
use crate::monitor_manager::get_primary_monitor;

fn set_window_properties(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let gtk_window = window.gtk_window()?;

    unsafe {
        gtk_utils::invoke_on_main(move || {
            gtk_window.set_resizable(false);
            gtk_window.set_decorated(false);
        });
    }

    Ok(())
}

pub async fn create_osd_window(
    app: &AppHandle,
    icon: &str,
    value: f64,
    maximum: f64,
    label: &str,
) -> Result<tauri::WebviewWindow, Box<dyn std::error::Error>> {
    let primary_monitor = get_primary_monitor(app).ok_or("No primary monitor found")?;
    let monitor_size = primary_monitor.size();
    let monitor_position = primary_monitor.position();

    let params = Serializer::new(String::new())
        .append_pair("icon", icon)
        .append_pair("value", &value.to_string())
        .append_pair("maximum", &maximum.to_string())
        .append_pair("label", label)
        .finish();

    let window = WebviewWindowBuilder::new(
        app,
        "osd_popup",
        WebviewUrl::App(format!("index.html#/apps/osd-popup?{}", params).into()),
    )
    .title("")
    .decorations(false)
    .transparent(true)
    .inner_size(220.0, 120.0)
    .visible(false)
    .skip_taskbar(true)
    .build()?;

    let complete_url = format!("{}/index.html#/apps/osd-popup?{}", get_app_url(), params);
    let url = Url::parse(&complete_url).expect("Failed to parse URL");
    let _ = window.navigate(url);

    let center_x = monitor_position.x + (monitor_size.width as i32 / 2) - (320 / 2);
    let center_y = monitor_position.y + (monitor_size.height as i32 / 3) - (140 / 2);

    window.set_position(tauri::PhysicalPosition { x: center_x, y: center_y })?;

    let _ = set_window_properties(&window);

    Ok(window)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 100 % (sin escala) el OSD queda centrado en X: a 1920 de ancho con una
    /// ventana de 220, el borde izquierdo va en (1920-220)/2 = 850. El bug que
    /// arreglamos restaba 320/2 = 160 en vez de 220/2, así que caía en 800: 50 px
    /// a la izquierda. La prueba fija 850 y, explícito, que ya no es 800.
    #[test]
    fn a_100_por_ciento_el_osd_se_centra_en_x_y_no_cae_en_el_viejo_desvio_de_50px() {
        let (x, _y) = osd_physical_position(
            (0, 0),
            (1920, 1080),
            1.0,
            OSD_LOGICAL_WIDTH,
            OSD_LOGICAL_HEIGHT,
        );

        assert_eq!(x, 850, "el OSD tiene que centrarse en 850 a 1920 de ancho");
        assert_ne!(x, 800, "800 era el desvío de 50 px del bug (restaba 320/2)");
    }

    /// En Y el OSD va en el tercio superior: su centro en height/3, de modo que
    /// el borde superior es height/3 - alto/2. A 1080 con alto 120: 360-60 = 300.
    /// El bug restaba 140/2 = 70 en vez de 120/2, dando 290.
    #[test]
    fn a_100_por_ciento_el_osd_se_ubica_en_el_tercio_superior_en_y() {
        let (_x, y) = osd_physical_position(
            (0, 0),
            (1920, 1080),
            1.0,
            OSD_LOGICAL_WIDTH,
            OSD_LOGICAL_HEIGHT,
        );

        assert_eq!(y, 300, "el borde superior va en height/3 - alto/2 = 300");
        assert_ne!(y, 290, "290 era el error en Y del bug (restaba 140/2)");
    }

    /// `inner_size` es lógico y el compositor lo escala por el factor del monitor,
    /// pero la posición se fija en físicos. A 2x la ventana de 220×120 mide 440×240
    /// físicos, así que a 3840×2160 el centro en X es (3840-440)/2 = 1700 y el
    /// borde superior en Y es 2160/3 - 240/2 = 720-120 = 600. El bug no escalaba el
    /// tamaño, así que en 2x se corría para el otro lado.
    #[test]
    fn a_2x_el_tamano_fisico_se_duplica_y_el_osd_sigue_centrado() {
        let (x, y) = osd_physical_position(
            (0, 0),
            (3840, 2160),
            2.0,
            OSD_LOGICAL_WIDTH,
            OSD_LOGICAL_HEIGHT,
        );

        assert_eq!(x, 1700, "a 2x el ancho físico es 440: (3840-440)/2 = 1700");
        assert_eq!(y, 600, "a 2x el alto físico es 240: 2160/3 - 240/2 = 600");
    }

    /// En multimonitor la geometría del monitor primario viene con offset: su
    /// esquina no es (0,0). El offset se suma tal cual a la posición. Un monitor
    /// 1920×1080 a la derecha, con origen en (1920,0), centra el OSD en
    /// 1920 + (1920-220)/2 = 2770 en X; el offset en Y es 0, así que Y no cambia.
    #[test]
    fn el_offset_del_monitor_primario_se_suma_a_la_posicion() {
        let (x, y) = osd_physical_position(
            (1920, 0),
            (1920, 1080),
            1.0,
            OSD_LOGICAL_WIDTH,
            OSD_LOGICAL_HEIGHT,
        );

        assert_eq!(
            x, 2770,
            "el offset del monitor (1920) se suma al centrado (850)"
        );
        assert_eq!(y, 300, "sin offset vertical, Y queda en el tercio superior");
    }

    /// Con escala fraccionaria el tamaño físico se redondea antes de centrar. A
    /// 1.5x la ventana de 220×120 mide round(330)×round(180) = 330×180 físicos:
    /// en 1920×1080 el centro en X es (1920-330)/2 = 795 y el borde superior en Y
    /// es 1080/3 - 180/2 = 360-90 = 270.
    #[test]
    fn con_escala_fraccionaria_el_tamano_fisico_se_redondea() {
        let (x, y) = osd_physical_position(
            (0, 0),
            (1920, 1080),
            1.5,
            OSD_LOGICAL_WIDTH,
            OSD_LOGICAL_HEIGHT,
        );

        assert_eq!(x, 795, "a 1.5x el ancho físico es 330: (1920-330)/2 = 795");
        assert_eq!(y, 270, "a 1.5x el alto físico es 180: 1080/3 - 180/2 = 270");
    }
}
