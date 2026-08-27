use super::app_icon;
use super::{wayfire_ipc::{get_wayfire_client, View}, WindowInfo, WindowManagerBackend};
use std::sync::mpsc::Sender;

pub struct WaylandManager {
}

impl WaylandManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {})
    }

    fn block_on_async<F, T>(f: F) -> Result<T, Box<dyn std::error::Error>>
    where
        F: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
    {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(f))
                .map_err(|e| -> Box<dyn std::error::Error> { e }),
            Err(_) => tauri::async_runtime::block_on(f)
                .map_err(|e| -> Box<dyn std::error::Error> { e }),
        }
    }

    /// Wayfire manda el texto `nil` cuando el cliente no dio el dato.
    fn field(value: Option<&str>) -> Option<&str> {
        value
            .map(str::trim)
            .filter(|value| !value.is_empty() && *value != "nil")
    }

    /// Sólo las ventanas de aplicación van al panel.
    ///
    /// Wayfire le pone `role` a cada superficie, y el que corresponde a una
    /// ventana de aplicación es `toplevel`: las superficies del propio
    /// escritorio —el panel, el fondo, el centro de control, los carteles de
    /// notificación— llegan como `desktop-environment`, y los menús y globos
    /// como `unmanaged`. Antes se decidía por el campo `type`, y esa lista no
    /// incluía `overlay`, que es lo que declara una superficie en la capa
    /// *overlay*: por eso cada notificación de vasak-flare aparecía en el panel
    /// como una aplicación abierta, con título «layer-shell».
    fn is_application_window(view: &View) -> bool {
        match Self::field(view.role.as_deref()) {
            Some(role) => role.eq_ignore_ascii_case("toplevel"),
            // Sin rol no se puede decidir por acá; deciden los demás filtros.
            None => true,
        }
    }

    /// Los espacios de nombres con que el escritorio declara sus superficies de
    /// capa —el `namespace` que se le pasa a `gtk-layer-shell`—, y con los que
    /// llegan como `app-id`.
    ///
    /// Es una lista exacta y no un prefijo `vasak-`: vasak-terminal,
    /// vasak-settings o vasak-file-manager son aplicaciones como cualquier otra
    /// y tienen que verse en el panel.
    const SHELL_NAMESPACES: [&'static str; 5] = [
        "vasak",
        "vasak-panel",
        "vasak-desktop",
        "vasak-control-center",
        "vasak-flare",
    ];

    fn is_shell_window(view: &View) -> bool {
        matches!(
            Self::field(view.app_id.as_deref()),
            Some(app_id) if Self::SHELL_NAMESPACES.contains(&app_id)
        )
    }

    /// Red de seguridad para cuando no llega `role`: las superficies de capa se
    /// reconocen por el `type` que Wayfire deriva de su capa.
    fn is_layer_shell_window(view: &View) -> bool {
        if Self::is_shell_window(view) {
            return true;
        }

        view
            .type_field
            .as_deref()
            .is_some_and(|value| {
                let lower = value.to_lowercase();
                matches!(
                    lower.as_str(),
                    "panel"
                        | "desktop"
                        | "dock"
                        | "background"
                        | "bottom"
                        | "top"
                        | "overlay"
                        | "layer-shell"
                        | "layershell"
                ) || lower.contains("layer-shell")
            })
    }

    /// El nombre del icono: la clave `Icon` de la entrada `.desktop` de la
    /// aplicación, y si no hay entrada, el `app-id` tal cual.
    fn icon_name(view: &View) -> String {
        Self::field(view.app_id.as_deref())
            .and_then(|app_id| {
                app_icon::icon_for_app_id(app_id)
                    .or_else(|| Some(app_icon::fallback_icon_name(app_id)))
            })
            .filter(|icon| !icon.is_empty())
            .unwrap_or_else(|| app_icon::FALLBACK_ICON.to_string())
    }

    fn view_to_window_info(view: &View) -> Option<WindowInfo> {
        if view.mapped == Some(false) {
            return None;
        }

        // Skip non-focusable windows (tooltips, popups)
        if view.focusable == Some(false) {
            return None;
        }

        if matches!(view.layer.as_deref(), Some("background") | Some("bottom")) {
            return None;
        }

        if !Self::is_application_window(view) {
            return None;
        }

        if Self::is_layer_shell_window(view) {
            return None;
        }

        let title = Self::field(view.title.as_deref()).unwrap_or_default().to_string();
        let icon = Self::icon_name(view);

        if title.is_empty() && icon == app_icon::FALLBACK_ICON {
            return None;
        }

        Some(WindowInfo {
            id: view.id.to_string(),
            title,
            is_minimized: view.minimized.unwrap_or(false),
            icon,
            demands_attention: None,
        })
    }
}

impl WindowManagerBackend for WaylandManager {
    fn get_window_list(&self) -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>> {
        let windows = Self::block_on_async(async {
            let client = get_wayfire_client().await.ok_or("Unable to connect to Wayfire IPC")?;
            let views = client.list_views_typed().await?;
            let mut windows: Vec<WindowInfo> = views.iter().filter_map(Self::view_to_window_info).collect();
            windows.sort_by(|left, right| {
                let l = left.id.parse::<u64>().unwrap_or(u64::MAX);
                let r = right.id.parse::<u64>().unwrap_or(u64::MAX);
                l.cmp(&r)
            });
            Result::<_, Box<dyn std::error::Error + Send + Sync>>::Ok(windows)
        })?;

        Ok(windows)
    }

    fn setup_event_monitoring(&mut self, tx: Sender<()>) -> Result<(), Box<dyn std::error::Error>> {
        let client = tauri::async_runtime::block_on(async {
            get_wayfire_client()
                .await
                .ok_or("Unable to connect to Wayfire IPC")
        })?;
        let mut receiver = client.subscribe();

        let _ = tx.send(());

        tauri::async_runtime::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(_) => {
                        let _ = tx.send(());
                    }
                    Err(err) => {
                        log::warn!("Wayfire event stream closed: {}", err);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    fn toggle_window(&self, win_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let view_id = win_id.parse::<u64>().map_err(|error| format!("invalid Wayfire view id {win_id}: {error}"))?;
        let view_id_i64 = i64::try_from(view_id).map_err(|error| format!("Wayfire view id out of range {win_id}: {error}"))?;

        Self::block_on_async(async move {
            let client = get_wayfire_client().await.ok_or("Unable to connect to Wayfire IPC")?;
            let views = client.list_views_typed().await?;
            let view = views
                .into_iter()
                .find(|candidate| candidate.id == view_id_i64)
                .ok_or_else(|| format!("Wayfire view not found: {view_id}"))?;

            if view.minimized.unwrap_or(false) {
                client.set_minimized(view_id, false).await?;
                client.set_focus(view_id).await.map(|_| ())
            } else if view.activated {
                client.set_minimized(view_id, true).await.map(|_| ())
            } else {
                client.set_focus(view_id).await.map(|_| ())
            }
        })?;

        Ok(())
    }
}

impl Default for WaylandManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize WaylandManager")
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Una vista como las que manda Wayfire, con lo mínimo para decidir.
    fn vista(id: i64, app_id: &str, title: &str, role: &str, type_field: &str, layer: &str) -> View {
        View {
            activated: false,
            app_id: Some(app_id.to_string()),
            base_geometry: None,
            bbox: None,
            focusable: Some(true),
            fullscreen: Some(false),
            geometry: None,
            id,
            last_focus_timestamp: None,
            layer: Some(layer.to_string()),
            mapped: Some(true),
            max_size: None,
            min_size: None,
            minimized: Some(false),
            output_id: None,
            output_name: None,
            parent: None,
            pid: None,
            role: Some(role.to_string()),
            sticky: Some(false),
            tiled_edges: None,
            title: Some(title.to_string()),
            type_field: Some(type_field.to_string()),
            wset_index: None,
        }
    }

    fn aplicacion(id: i64, app_id: &str, title: &str) -> View {
        vista(id, app_id, title, "toplevel", "toplevel", "workspace")
    }

    #[test]
    fn un_cartel_de_notificacion_no_es_una_aplicacion_abierta() {
        // La regresión: el cartel de vasak-flare vive en la capa *overlay*, y
        // el filtro por `type` no conocía ese valor, así que cada notificación
        // se sumaba al panel como una aplicación abierta llamada «layer-shell».
        let cartel = vista(
            2196,
            "vasak-flare",
            "layer-shell",
            "desktop-environment",
            "overlay",
            "overlay",
        );
        assert_eq!(WaylandManager::view_to_window_info(&cartel), None);
    }

    #[test]
    fn ninguna_superficie_del_escritorio_va_al_panel() {
        let superficies = [
            vista(1749, "vasak-desktop", "layer-shell", "desktop-environment", "background", "background"),
            vista(1751, "vasak-panel", "layer-shell", "desktop-environment", "panel", "top"),
            vista(1800, "vasak", "layer-shell", "desktop-environment", "panel", "top"),
            vista(1801, "vasak-control-center", "layer-shell", "desktop-environment", "overlay", "overlay"),
            vista(3, "nil", "nil", "unmanaged", "unmanaged", "none"),
        ];

        for superficie in &superficies {
            assert_eq!(
                WaylandManager::view_to_window_info(superficie),
                None,
                "{:?} llegó al panel",
                superficie.app_id
            );
        }
    }

    #[test]
    fn las_aplicaciones_del_sistema_si_van_al_panel() {
        // El otro lado del filtro anterior: que se descarten las superficies
        // del escritorio no puede llevarse puestas las aplicaciones propias,
        // que comparten el prefijo del nombre.
        for app_id in ["vasak-terminal", "vasak-settings", "vasak-file-manager"] {
            let ventana = WaylandManager::view_to_window_info(&aplicacion(2066, app_id, "Título"));
            assert!(ventana.is_some(), "{app_id} no llegó al panel");
        }
    }

    #[test]
    fn el_icono_no_se_recorta_por_el_punto() {
        // Sin entrada `.desktop` a mano, el nombre de reserva tiene que ser el
        // `app-id` completo: recortarlo daba `desktop` para Telegram, que en el
        // tema de iconos es la carpeta del escritorio.
        //
        // El `app-id` es propio de esta prueba, así que no hace falta vaciar lo
        // memorizado —y no conviene: es global, y las pruebas corren en
        // paralelo—.
        let telegram = aplicacion(232, "no.instalado.telegram", "Programación y Dev");
        let ventana = WaylandManager::view_to_window_info(&telegram).expect("debería listarse");
        assert_eq!(ventana.icon, "no.instalado.telegram");
        assert_eq!(ventana.id, "232");
        assert_eq!(ventana.title, "Programación y Dev");
    }

    #[test]
    fn una_ventana_sin_datos_no_se_lista() {
        // `nil` es lo que manda Wayfire cuando el cliente no dio el dato, y no
        // un título ni un nombre de icono.
        let vacia = aplicacion(10, "nil", "nil");
        assert_eq!(WaylandManager::view_to_window_info(&vacia), None);
    }

    #[test]
    fn una_ventana_sin_mapear_o_sin_foco_no_se_lista() {
        let mut sin_mapear = aplicacion(11, "discord", "Discord");
        sin_mapear.mapped = Some(false);
        assert_eq!(WaylandManager::view_to_window_info(&sin_mapear), None);

        let mut sin_foco = aplicacion(12, "discord", "Discord");
        sin_foco.focusable = Some(false);
        assert_eq!(WaylandManager::view_to_window_info(&sin_foco), None);
    }

    #[test]
    fn una_ventana_minimizada_se_lista_como_tal() {
        let mut minimizada = aplicacion(16, "discord", "Discord");
        minimizada.minimized = Some(true);
        let ventana = WaylandManager::view_to_window_info(&minimizada).expect("debería listarse");
        assert!(ventana.is_minimized);
    }
}

/// Sobre una respuesta real de `window-rules/list-views`, capturada de esta
/// sesión con el panel mostrando de más y de menos.
#[cfg(test)]
mod pruebas_con_captura_real {
    use super::*;

    /// Lo que devolvió el compositor el día que se arregló esto: un cartel de
    /// notificación en la capa *overlay*, las superficies del escritorio, una
    /// ventana suelta de vasak-desktop y las aplicaciones de verdad.
    const CAPTURA: &str = include_str!("fixtures/wayfire-list-views.json");

    fn ventanas() -> Vec<WindowInfo> {
        let views: Vec<View> =
            serde_json::from_str(CAPTURA).expect("la captura del compositor tiene que parsear");
        views
            .iter()
            .filter_map(WaylandManager::view_to_window_info)
            .collect()
    }

    #[test]
    fn el_panel_lista_las_aplicaciones_y_nada_mas() {
        let ventanas = ventanas();
        let ids: Vec<&str> = ventanas.iter().map(|w| w.id.as_str()).collect();

        // Discord, Claude, Telegram, Chrome y la terminal.
        assert_eq!(ids, vec!["16", "21", "232", "1757", "2066"]);
    }

    #[test]
    fn telegram_esta_y_con_su_icono() {
        let telegram = ventanas()
            .into_iter()
            .find(|w| w.id == "232")
            .expect("Telegram tiene que estar en el panel");

        // `desktop` era lo que salía de recortar `org.telegram.desktop` por el
        // punto, y en el tema de iconos es la carpeta del escritorio.
        assert_ne!(telegram.icon, "desktop");
        assert!(
            telegram.icon.contains("telegram"),
            "icono inesperado: {}",
            telegram.icon
        );
    }
}

/// Contra el compositor de verdad, con la sesión andando. Se corre a mano
/// —`cargo test -- --ignored ventanas_de_la_sesion`— porque necesita un Wayfire
/// con su socket de IPC, que en una máquina de integración no está.
///
/// Sirve para lo que ninguna captura fija puede: ver qué recibe el panel ahora
/// mismo, con las aplicaciones que estén abiertas y con un cartel de
/// notificación en pantalla.
#[cfg(test)]
mod prueba_en_vivo {
    use super::*;

    #[ignore = "necesita una sesión de Wayfire andando"]
    #[tokio::test]
    async fn ventanas_de_la_sesion() {
        let client = get_wayfire_client()
            .await
            .expect("sin socket de Wayfire no hay nada que probar");
        let views = client.list_views_typed().await.expect("list-views falló");

        let ventanas: Vec<WindowInfo> = views
            .iter()
            .filter_map(WaylandManager::view_to_window_info)
            .collect();

        for ventana in &ventanas {
            println!("{} · icono {} · {}", ventana.id, ventana.icon, ventana.title);
        }

        // Ninguna superficie del escritorio: ni el panel, ni el fondo, ni los
        // carteles de notificación.
        for ventana in &ventanas {
            assert_ne!(ventana.title, "layer-shell", "una superficie de capa llegó al panel");
        }

        // Y ningún icono adivinado por el último tramo del identificador.
        for ventana in &ventanas {
            assert_ne!(ventana.icon, "desktop", "{} quedó con el icono de la carpeta", ventana.title);
        }
    }
}
