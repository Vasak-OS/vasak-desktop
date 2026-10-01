//! Lo que un StatusNotifierItem dice de sí mismo, leído como dice la
//! especificación (freedesktop.org, «StatusNotifierItem») y como lo mandan las
//! aplicaciones de verdad.
//!
//! Dos cosas que la especificación no dice y la práctica sí:
//!
//! - **Una cadena vacía es «no viene».** Chromium, Electron y Qt responden
//!   `OverlayIconName = ""`, `Title = ""`, `AttentionIconName = ""`, y
//!   `IconPixmap` o `OverlayIconPixmap` con un arreglo vacío. Nada de eso se
//!   reenvía a la vista.
//! - **Una propiedad puede faltar o fallar.** Chromium responde con error a
//!   `IconName` e `IconThemePath`; se trata igual que vacía.
//!
//! Las señales (`NewIcon`, `NewOverlayIcon`, `NewAttentionIcon`, `NewToolTip`,
//! `NewTitle`, `NewStatus`) no traen el valor nuevo —salvo `NewStatus`—: piden
//! volver a leerlo. [`Refresh::for_signal`] dice qué hay que releer con cada
//! una, para no pedirle quince propiedades al elemento por cada parpadeo.

use crate::structs::{TrayCategory, TrayIcon, TrayItem, TrayStatus, TrayTooltip};
use crate::tray::pixmap::{best_pixmap_png_base64, menu_icon_png_base64, Pixmap};
use crate::tray::sni_item::SniItemProxy;

/// Las interfaces con las que emiten los elementos. La especificación dice
/// `org.freedesktop.StatusNotifierItem`; en la práctica todos —KDE, Qt,
/// Chromium, libayatana— usan `org.kde.StatusNotifierItem`.
pub const SNI_INTERFACES: [&str; 2] = [
    "org.kde.StatusNotifierItem",
    "org.freedesktop.StatusNotifierItem",
];

/// Qué hay que releer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refresh {
    Icon,
    OverlayIcon,
    AttentionIcon,
    ToolTip,
    Title,
    /// `NewStatus` trae el estado como argumento: no hay que leer nada.
    Status,
}

impl Refresh {
    pub fn for_signal(member: &str) -> Option<Self> {
        match member {
            "NewIcon" => Some(Refresh::Icon),
            "NewOverlayIcon" => Some(Refresh::OverlayIcon),
            "NewAttentionIcon" => Some(Refresh::AttentionIcon),
            "NewToolTip" => Some(Refresh::ToolTip),
            "NewTitle" => Some(Refresh::Title),
            "NewStatus" => Some(Refresh::Status),
            _ => None,
        }
    }
}

pub fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.trim().is_empty())
}

/// La descripción del globo puede traer `<b>`, `<i>`, `<u>`, `<a>` e `<img>`.
/// La especificación permite filtrarlos («sin perder el contenido»), que es lo
/// que se hace: la vista la muestra como texto, nunca como HTML de otro
/// proceso. Las entidades comunes se decodifican y un `<br>` es un salto.
pub fn strip_markup(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            for t in chars.by_ref() {
                if t == '>' {
                    break;
                }
                tag.push(t);
            }
            let name: String = tag
                .trim_start_matches('/')
                .trim()
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            if name == "br" || name == "p" {
                out.push('\n');
            }
        } else {
            out.push(c);
        }
    }
    let decoded = out
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&");
    decoded
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// El icono por nombre, buscado primero en `IconThemePath` si el elemento lo
/// da: ahí ponen sus iconos propios las aplicaciones que no los instalan en el
/// tema (Nextcloud, KDE Connect desde un AppImage…). Encontrado ahí, es el
/// dibujo de la aplicación y viaja como mapa de bits; si no, queda el nombre
/// para el tema del sistema.
pub fn icon_from_parts(
    name: Option<String>,
    pixmaps: Option<Vec<Pixmap>>,
    theme_path: Option<&str>,
) -> Option<TrayIcon> {
    let name = non_empty(name);
    let mut data = pixmaps.as_deref().and_then(best_pixmap_png_base64);
    if data.is_none() {
        if let (Some(name), Some(path)) = (name.as_deref(), theme_path) {
            data = find_in_theme_path(path, name);
        }
    }
    TrayIcon { name, data }.non_empty()
}

/// Los tamaños que se prueban en un `IconThemePath`, del que mejor se achica
/// al que peor.
const THEME_PATH_SIZES: [&str; 7] = [
    "48x48", "64x64", "32x32", "128x128", "24x24", "22x22", "16x16",
];

/// Busca `<nombre>.png` en un `IconThemePath`, **sólo** en los lugares donde lo
/// pone un tema de iconos: la carpeta misma, `<tamaño>/<contexto>/` y
/// `hicolor/<tamaño>/<contexto>/`. No se recorre la carpeta: el camino lo da
/// cualquier cliente del bus, y un `/` haría leer medio disco por cada icono.
/// Son, como mucho, 29 `stat`.
pub fn find_in_theme_path(path: &str, name: &str) -> Option<String> {
    if path.trim().is_empty() || name.is_empty() || name.contains('/') || name.contains("..") {
        return None;
    }
    let root = std::path::Path::new(path);
    let file = format!("{name}.png");
    let mut candidates = vec![root.join(&file)];
    for base in [root.to_path_buf(), root.join("hicolor")] {
        for size in THEME_PATH_SIZES {
            for context in ["apps", "status"] {
                candidates.push(base.join(size).join(context).join(&file));
            }
        }
    }
    let found = candidates.into_iter().find(|c| c.is_file())?;
    let bytes = std::fs::read(found).ok()?;
    menu_icon_png_base64(&bytes)
}

/// [`icon_from_parts`] fuera del hilo del runtime: convierte mapas de bits y
/// puede tocar el disco.
async fn resolve_icon(
    name: Option<String>,
    pixmaps: Option<Vec<Pixmap>>,
    theme_path: Option<String>,
) -> Option<TrayIcon> {
    tokio::task::spawn_blocking(move || icon_from_parts(name, pixmaps, theme_path.as_deref()))
        .await
        .ok()
        .flatten()
}

/// El globo, o `None` si vino todo vacío.
pub fn tooltip_from_parts(
    icon_name: String,
    pixmaps: Vec<Pixmap>,
    title: String,
    description: String,
) -> Option<TrayTooltip> {
    let tooltip = TrayTooltip {
        icon: icon_from_parts(Some(icon_name), Some(pixmaps), None),
        title: non_empty(Some(strip_markup(&title))),
        description: non_empty(Some(strip_markup(&description))),
    };
    (tooltip != TrayTooltip::default()).then_some(tooltip)
}

fn parse_category(value: &str) -> TrayCategory {
    match value {
        "Communications" => TrayCategory::Communications,
        "SystemServices" => TrayCategory::SystemServices,
        "Hardware" => TrayCategory::Hardware,
        _ => TrayCategory::ApplicationStatus,
    }
}

async fn read_icon(
    proxy: &SniItemProxy<'_>,
    theme_path: Option<String>,
) -> (Option<String>, Option<String>) {
    let icon = resolve_icon(
        proxy.icon_name().await.ok(),
        proxy.icon_pixmap().await.ok(),
        theme_path,
    )
    .await;
    match icon {
        Some(TrayIcon { name, data }) => (name, data),
        None => (None, None),
    }
}

async fn read_overlay(proxy: &SniItemProxy<'_>, theme_path: Option<String>) -> Option<TrayIcon> {
    resolve_icon(
        proxy.overlay_icon_name().await.ok(),
        proxy.overlay_icon_pixmap().await.ok(),
        theme_path,
    )
    .await
}

async fn read_attention(
    proxy: &SniItemProxy<'_>,
    theme_path: Option<String>,
) -> (Option<TrayIcon>, Option<String>) {
    let icon = resolve_icon(
        proxy.attention_icon_name().await.ok(),
        proxy.attention_icon_pixmap().await.ok(),
        theme_path,
    )
    .await;
    (icon, non_empty(proxy.attention_movie_name().await.ok()))
}

async fn read_tooltip(proxy: &SniItemProxy<'_>) -> Option<TrayTooltip> {
    let (name, pixmaps, title, description) = proxy.tool_tip().await.ok()?;
    tokio::task::spawn_blocking(move || tooltip_from_parts(name, pixmaps, title, description))
        .await
        .ok()
        .flatten()
}

async fn read_theme_path(proxy: &SniItemProxy<'_>) -> Option<String> {
    non_empty(proxy.icon_theme_path().await.ok())
}

/// Lee todo el elemento.
pub async fn read_item(
    proxy: &SniItemProxy<'_>,
    service_name: &str,
    bus_name: &str,
    object_path: &str,
) -> TrayItem {
    let id = non_empty(proxy.id().await.ok()).unwrap_or_else(|| service_name.to_string());
    let theme_path = read_theme_path(proxy).await;
    let (icon_name, icon_data) = read_icon(proxy, theme_path.clone()).await;
    let (attention_icon, attention_movie_name) = read_attention(proxy, theme_path.clone()).await;

    TrayItem {
        id,
        service_name: service_name.to_string(),
        bus_name: Some(bus_name.to_string()),
        icon_name,
        icon_data,
        overlay_icon: read_overlay(proxy, theme_path).await,
        attention_icon,
        attention_movie_name,
        title: non_empty(proxy.title().await.ok()),
        tooltip: read_tooltip(proxy).await,
        status: TrayStatus::parse(&proxy.status().await.unwrap_or_default()),
        category: parse_category(&proxy.category().await.unwrap_or_default()),
        menu_path: None,
        item_is_menu: proxy.item_is_menu().await.unwrap_or(false),
        launcher: None,
        object_path: object_path.to_string(),
        unique_name: None,
        pid: None,
    }
}

/// Relee lo que pide una señal y lo deja en `item`.
///
/// `IconThemePath` se pide sólo para las señales de iconos: el título o el
/// globo no lo usan.
pub async fn refresh_item(proxy: &SniItemProxy<'_>, item: &mut TrayItem, refresh: Refresh) {
    match refresh {
        Refresh::Icon => {
            let (name, data) = read_icon(proxy, read_theme_path(proxy).await).await;
            item.icon_name = name;
            item.icon_data = data;
        }
        Refresh::OverlayIcon => {
            item.overlay_icon = read_overlay(proxy, read_theme_path(proxy).await).await
        }
        Refresh::AttentionIcon => {
            let (icon, movie) = read_attention(proxy, read_theme_path(proxy).await).await;
            item.attention_icon = icon;
            item.attention_movie_name = movie;
        }
        Refresh::ToolTip => item.tooltip = read_tooltip(proxy).await,
        Refresh::Title => item.title = non_empty(proxy.title().await.ok()),
        Refresh::Status => {
            item.status = TrayStatus::parse(&proxy.status().await.unwrap_or_default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tray::pixmap::tests::{png_bytes, solid};

    #[test]
    fn cada_senal_relee_lo_suyo() {
        assert_eq!(Refresh::for_signal("NewIcon"), Some(Refresh::Icon));
        assert_eq!(
            Refresh::for_signal("NewOverlayIcon"),
            Some(Refresh::OverlayIcon)
        );
        assert_eq!(
            Refresh::for_signal("NewAttentionIcon"),
            Some(Refresh::AttentionIcon)
        );
        assert_eq!(Refresh::for_signal("NewToolTip"), Some(Refresh::ToolTip));
        assert_eq!(Refresh::for_signal("NewTitle"), Some(Refresh::Title));
        assert_eq!(Refresh::for_signal("NewStatus"), Some(Refresh::Status));
        assert_eq!(Refresh::for_signal("NewMenu"), None);
    }

    #[test]
    fn el_marcado_del_globo_se_filtra_sin_perder_el_texto() {
        assert_eq!(
            strip_markup("<b>3</b> mensajes &amp; <i>1</i> llamada"),
            "3 mensajes & 1 llamada"
        );
        assert_eq!(
            strip_markup("Línea uno<br/>Línea dos"),
            "Línea uno\nLínea dos"
        );
        assert_eq!(
            strip_markup("<a href=\"x\">enlace</a> <img src=\"/a.png\" alt=\"a\"/>"),
            "enlace"
        );
        assert_eq!(strip_markup("&lt;script&gt;"), "<script>");
        assert_eq!(strip_markup("   "), "");
    }

    #[test]
    fn lo_vacio_no_viene() {
        assert_eq!(
            icon_from_parts(Some(String::new()), Some(vec![]), None),
            None
        );
        assert_eq!(icon_from_parts(None, None, None), None);
        assert_eq!(
            tooltip_from_parts(String::new(), vec![], String::new(), String::new()),
            None
        );
    }

    #[test]
    fn un_icono_por_nombre_por_mapa_o_los_dos() {
        let by_name = icon_from_parts(Some("telegram-panel".into()), Some(vec![]), None).unwrap();
        assert_eq!(
            (by_name.name.as_deref(), by_name.data.is_some()),
            (Some("telegram-panel"), false)
        );
        let by_pixmap =
            icon_from_parts(None, Some(vec![solid(16, 16, [255, 1, 2, 3])]), None).unwrap();
        assert!(by_pixmap.name.is_none() && by_pixmap.data.is_some());
        let both = icon_from_parts(
            Some("x".into()),
            Some(vec![solid(16, 16, [255, 1, 2, 3])]),
            None,
        )
        .unwrap();
        assert!(both.name.is_some() && both.data.is_some());
    }

    #[test]
    fn el_globo_de_chromium_es_solo_un_titulo() {
        // Lo que manda Discord, medido: ("", [], "Discord", "").
        let tooltip =
            tooltip_from_parts(String::new(), vec![], "Discord".into(), String::new()).unwrap();
        assert_eq!(
            tooltip,
            TrayTooltip {
                icon: None,
                title: Some("Discord".into()),
                description: None
            }
        );
    }

    #[test]
    fn el_globo_completo_trae_icono_titulo_y_descripcion() {
        let tooltip = tooltip_from_parts(
            "nextcloud".into(),
            vec![],
            "Nextcloud".into(),
            "Sincronizando <b>3</b> archivos".into(),
        )
        .unwrap();
        assert_eq!(tooltip.icon.unwrap().name.as_deref(), Some("nextcloud"));
        assert_eq!(
            tooltip.description.as_deref(),
            Some("Sincronizando 3 archivos")
        );
    }

    #[test]
    fn el_icono_propio_se_busca_en_icon_theme_path() {
        let dir = std::env::temp_dir().join(format!("vasak-tray-theme-{}", std::process::id()));
        let nested = dir.join("hicolor/22x22/apps");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("mi-app.png"), png_bytes(22, 22)).unwrap();
        let icon = icon_from_parts(Some("mi-app".into()), None, dir.to_str()).unwrap();
        assert!(icon.data.is_some());
        // Lo que no está queda como nombre para el tema del sistema.
        let other = icon_from_parts(Some("otra".into()), None, dir.to_str()).unwrap();
        assert_eq!((other.name.as_deref(), other.data), (Some("otra"), None));
        // Fuera de los lugares de un tema no se busca: la carpeta no se recorre.
        let odd = dir.join("cualquier/cosa/honda");
        std::fs::create_dir_all(&odd).unwrap();
        std::fs::write(odd.join("escondido.png"), png_bytes(22, 22)).unwrap();
        assert_eq!(find_in_theme_path(dir.to_str().unwrap(), "escondido"), None);
        // En la raíz de la carpeta, sí.
        std::fs::write(dir.join("suelto.png"), png_bytes(16, 16)).unwrap();
        assert!(find_in_theme_path(dir.to_str().unwrap(), "suelto").is_some());
        // Un nombre con barras no sale de la carpeta.
        assert_eq!(find_in_theme_path(dir.to_str().unwrap(), "../x"), None);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
