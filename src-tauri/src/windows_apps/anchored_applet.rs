//! Los applets del panel, anclados al botón que los abre.
//!
//! Antes cada applet era una ventana común que se centraba en el monitor con
//! `set_position`. En Wayland eso no hace nada —el cliente no decide dónde queda
//! su ventana—, así que el applet aparecía donde quisiera el compositor, lejos
//! del botón que se había tocado. Ahora es una superficie de capa pegada al
//! panel: debajo del botón con el panel arriba, encima con el panel abajo y al
//! costado con el panel a un lado, corrida lo justo para no salirse del monitor.
//!
//! Hay **un solo camino** para abrir un applet, [`toggle_anchored_applet`], y
//! una sola tabla que dice cuáles hay y cuánto miden, [`APPLETS`]. Un applet
//! nuevo es una fila en esa tabla y una ruta en la interfaz.
//!
//! # Uno por vez
//!
//! Abrir otro cierra el que estaba; tocar el mismo botón con su applet abierto lo
//! cierra. El panel se entera por `applet-changed`, que se emite desde acá al
//! abrir y al cerrar —cierre quien cierre—, y con eso realza el botón. No lo
//! adivina: un applet también se cierra con Escape o al perder el foco, y el
//! panel no ve ninguna de las dos cosas.
//!
//! # Sólo en el hilo principal de GTK
//!
//! Todo lo de acá toca superficies de GTK, que no se pueden tocar desde otro
//! hilo; el estado de qué está abierto vive en un `thread_local` de ese hilo.
//! [`toggle_anchored_applet`] y [`dismiss_anchored_applet`] se encargan de llegar
//! ahí solos, así que se pueden llamar desde un comando `async`.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk_layer_shell::{KeyboardMode, Layer};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::logger::{log_error, log_info};
use crate::monitor_manager::{find_gdk_monitor, get_primary_monitor};
use crate::panel_position::{self, PanelPosition, PANEL_THICKNESS, SCREEN_MARGIN};
use crate::windows_apps::shell_layer::{
    destroy_layer_windows, hide_layer_window, layer_window_exists, layer_window_visible,
    relocate_layer_window, show_layer_window, spawn_layer_window, Geometry, LayerSpec,
};

/// Lo que se aparta el applet del borde interno del panel.
pub const PANEL_GAP: i32 = 8;

/// Cuánto dura la salida en la página, antes de esconder la superficie.
///
/// Un poco más que la animación de `AppletPopover.vue` (120 ms), para que el
/// último cuadro llegue a dibujarse y la superficie no se corte a medio fundir.
const LEAVE_DURATION: Duration = Duration::from_millis(140);

/// Cuánto después de un cierre por Escape o por foco un clic en el mismo botón
/// se toma como parte de ese cierre.
///
/// Tocar el botón del applet abierto le saca el foco antes de que llegue el
/// clic: el applet empieza a cerrarse, y el comando del botón lo encontraría
/// cerrado y lo volvería a abrir. Es la trampa en la que ya había caído el menú.
const REOPEN_GUARD: Duration = Duration::from_millis(350);

/// Todas las superficies de applet llevan este prefijo en la etiqueta.
const LABEL_PREFIX: &str = "applet_";

/// Un applet: cómo se llama, qué ruta dibuja y cuánto mide.
///
/// El tamaño lo fija Rust, una constante por applet. Si algún día un applet
/// necesita informar su tamaño después de montarse —una lista que varía mucho—,
/// es este campo el que pasa a ser el inicial, y el redimensionado en caliente
/// va por [`relocate_layer_window`], que ya lo acepta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppletSpec {
    /// Lo que manda la interfaz para pedirlo, y lo que vuelve en
    /// `applet-changed`.
    pub id: &'static str,
    /// La ruta de la interfaz, debajo de `/applets/`.
    pub route: &'static str,
    /// Ancho y alto, en píxeles lógicos.
    pub size: (f64, f64),
}

impl AppletSpec {
    pub fn label(&self) -> String {
        format!("{LABEL_PREFIX}{}", self.id)
    }
}

/// Los applets que hay.
// Una fila por applet, sin que `rustfmt` la abra en cinco: es una tabla, y
// leída como tabla se ve de un vistazo qué applets hay y cuánto mide cada uno.
// Abierta, cada fila era el mismo bloque de cinco líneas y el analizador de
// duplicados la contaba como código copiado.
#[rustfmt::skip]
pub const APPLETS: &[AppletSpec] = &[
    AppletSpec { id: "bluetooth", route: "bluetooth", size: (700.0, 620.0) },
    AppletSpec { id: "network", route: "network", size: (700.0, 620.0) },
    AppletSpec { id: "audio", route: "audio", size: (700.0, 620.0) },
    // El menú de un icono de la bandeja. Los datos los deja `open_tray_popup`
    // antes de abrirlo, y con ellos el tamaño (`tray_menu_size`): esto es sólo
    // el de un menú vacío.
    AppletSpec { id: "tray", route: "tray-popup", size: (280.0, 42.0) },
    // Quién usa la cámara, el micrófono y la pantalla. Alcanza para las tres
    // listas con varias aplicaciones en cada una sin desplazar en el caso
    // normal, que es una o dos.
    AppletSpec { id: "privacy", route: "privacy", size: (420.0, 420.0) },
    // Setenta recursos no entran igual, pero los que piden autorización sí, y
    // de eso se trata la pantalla.
    AppletSpec { id: "twingate", route: "twingate", size: (480.0, 560.0) },
    // El reproductor que se despliega desde el control de música: el disco,
    // los datos de la pista, la barra y el transporte. Cuatrocientos de ancho
    // como en la referencia; el alto es el de la tarjeta sin ecualizador —que
    // todavía no existe— con el título en dos líneas y los dos chips en dos
    // renglones, que es el caso más alto que se dibuja hoy (238 de tarjeta más
    // el relleno del contenedor). Cuando llegue el ecualizador, este es el
    // número que crece.
    AppletSpec { id: "music", route: "music", size: (400.0, 272.0) },
];

pub fn applet_spec(id: &str) -> Option<&'static AppletSpec> {
    APPLETS.iter().find(|spec| spec.id == id)
}

/// El rectángulo del botón que abrió el applet, tal como lo mide el panel con
/// `getBoundingClientRect()`: en píxeles lógicos y relativo a la superficie del
/// panel.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct AnchorRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Dónde queda un applet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Bordes anclados: (izquierda, derecha, arriba, abajo).
    pub anchors: (bool, bool, bool, bool),
    /// Separación de cada borde, en el mismo orden.
    pub margins: (i32, i32, i32, i32),
    /// El tamaño con que se abre: el pedido, o menos si no entra.
    pub size: (f64, f64),
    /// Dónde queda el centro del botón a lo largo del applet, medido desde su
    /// borde izquierdo (panel arriba o abajo) o desde el de arriba (a los
    /// costados). Es de donde crece la animación de entrada.
    pub origin: f64,
}

/// Lo que recibe la página al mostrarse: de qué lado está el panel y dónde
/// quedó el botón. Con eso arma el `transform-origin`.
#[derive(Debug, Clone, Serialize)]
pub struct AppletShown {
    pub applet: &'static str,
    pub side: &'static str,
    pub origin: f64,
}

/// Lo que recibe el panel cuando cambia el applet abierto.
#[derive(Debug, Clone, Serialize)]
pub struct AppletChanged {
    pub applet: Option<&'static str>,
}

/// Lleva `value` a `[min, max]`; si el intervalo está vacío, gana `min`.
///
/// `f64::clamp` entra en pánico con un intervalo al revés, y acá eso pasa de
/// verdad: es el applet más grande que el espacio que hay.
fn clamp_to(value: f64, min: f64, max: f64) -> f64 {
    value.min(max).max(min)
}

/// Dónde se abre el applet: anclas, márgenes, tamaño y origen de la animación.
///
/// - `anchor`: el botón, en píxeles lógicos del panel. `None` cuando lo que lo
///   abre no está en el panel (el centro de control): entonces va centrado en el
///   eje del panel.
/// - `side`: de qué lado de la pantalla está el panel.
/// - `requested`: lo que mide el applet.
/// - `monitor`: lo que mide el monitor del panel, en píxeles lógicos.
///
/// El applet se aparta [`PANEL_GAP`] del borde interno del panel y queda
/// centrado sobre el botón a lo largo del panel; si no entra, se corre hasta
/// quedar a [`SCREEN_MARGIN`] del borde del monitor, que es el margen del resto
/// del escritorio. Si es más grande que el espacio que hay, se achica: una
/// superficie que se sale del monitor la recorta el compositor sin avisar.
///
/// Las superficies de applet no reservan espacio (`exclusive_zone = -1`), así
/// que los márgenes se miden desde el borde del monitor y no desde donde
/// termina el panel: el grosor del panel se suma a mano.
pub fn place_applet(
    anchor: Option<&AnchorRect>,
    side: PanelPosition,
    requested: (f64, f64),
    monitor: (f64, f64),
) -> Placement {
    let (screen_width, screen_height) = monitor;
    let thickness = PANEL_THICKNESS as f64;
    let gap = PANEL_GAP as f64;
    let margin = SCREEN_MARGIN as f64;
    let (panel_x, panel_y) = side.origin(screen_width, screen_height);
    let away_from_panel = PANEL_THICKNESS + PANEL_GAP;

    if side.is_vertical() {
        let width = requested
            .0
            .min(screen_width - thickness - gap - margin)
            .max(1.0);
        let height = requested.1.min(screen_height - 2.0 * margin).max(1.0);

        let center = anchor
            .map(|rect| panel_y + rect.y + rect.height / 2.0)
            .unwrap_or(screen_height / 2.0);
        let top = clamp_to(
            center - height / 2.0,
            margin,
            screen_height - margin - height,
        );
        let origin = clamp_to(center - top, 0.0, height);
        let top = top.round() as i32;

        let (anchors, margins) = if side == PanelPosition::Left {
            ((true, false, true, false), (away_from_panel, 0, top, 0))
        } else {
            ((false, true, true, false), (0, away_from_panel, top, 0))
        };

        Placement {
            anchors,
            margins,
            size: (width, height),
            origin,
        }
    } else {
        let width = requested.0.min(screen_width - 2.0 * margin).max(1.0);
        let height = requested
            .1
            .min(screen_height - thickness - gap - margin)
            .max(1.0);

        let center = anchor
            .map(|rect| panel_x + rect.x + rect.width / 2.0)
            .unwrap_or(screen_width / 2.0);
        let left = clamp_to(center - width / 2.0, margin, screen_width - margin - width);
        let origin = clamp_to(center - left, 0.0, width);
        let left = left.round() as i32;

        let (anchors, margins) = if side == PanelPosition::Top {
            ((true, false, true, false), (left, 0, away_from_panel, 0))
        } else {
            ((true, false, false, true), (left, 0, 0, away_from_panel))
        };

        Placement {
            anchors,
            margins,
            size: (width, height),
            origin,
        }
    }
}

/// Si un clic en el botón de `id` tiene que cerrar el applet en vez de abrirlo,
/// porque ese mismo applet se acaba de empezar a cerrar por Escape o por foco.
fn closes_a_recent_dismissal(
    recent: Option<(&'static str, Instant)>,
    id: &str,
    now: Instant,
) -> bool {
    matches!(recent, Some((dismissed, at)) if dismissed == id && now.duration_since(at) < REOPEN_GUARD)
}

/// Qué está abierto, en el hilo principal de GTK.
#[derive(Default)]
struct AppletState {
    /// El applet a la vista.
    open: Option<&'static str>,
    /// El último que empezó a cerrarse por Escape o por foco, y cuándo.
    dismissed: Option<(&'static str, Instant)>,
    /// Sube con cada apertura y con cada cierre: un temporizador de salida que
    /// encuentra otro número sabe que llegó tarde y no esconde nada.
    generation: u64,
}

thread_local! {
    static STATE: RefCell<AppletState> = RefCell::new(AppletState::default());
}

fn announce(app: &AppHandle, applet: Option<&'static str>) {
    if let Err(error) = app.emit("applet-changed", AppletChanged { applet }) {
        log_error(&format!("[applet] no se pudo avisar al panel: {error}"));
    }
}

/// Abre el applet `id` debajo del botón `anchor`, o lo cierra si ya estaba
/// abierto.
///
/// Se puede llamar desde cualquier hilo: el trabajo se hace en el principal.
pub fn toggle_anchored_applet(
    app: &AppHandle,
    id: &str,
    anchor: Option<AnchorRect>,
) -> Result<(), String> {
    let spec = applet_spec(id).ok_or_else(|| format!("no hay ningún applet «{id}»"))?;
    let handle = app.clone();

    app.run_on_main_thread(move || toggle_on_main(&handle, spec, anchor))
        .map_err(|error| format!("no se pudo llegar al hilo principal: {error}"))
}

/// Abre el applet `id`, esté o no abierto: si ya estaba, lo vuelve a ubicar.
///
/// Es lo que usa la bandeja, donde el mismo applet muestra el menú de otro
/// icono cada vez. `size` reemplaza al del [`AppletSpec`] cuando el contenido
/// se conoce antes de abrir: un menú de tres entradas no ocupa lo mismo que
/// uno de veinte.
pub fn open_anchored_applet(
    app: &AppHandle,
    id: &str,
    anchor: Option<AnchorRect>,
    size: Option<(f64, f64)>,
) -> Result<(), String> {
    let spec = applet_spec(id).ok_or_else(|| format!("no hay ningún applet «{id}»"))?;
    let handle = app.clone();

    app.run_on_main_thread(move || open_on_main(&handle, spec, anchor.as_ref(), size))
        .map_err(|error| format!("no se pudo llegar al hilo principal: {error}"))
}

fn toggle_on_main(app: &AppHandle, spec: &'static AppletSpec, anchor: Option<AnchorRect>) {
    let label = spec.label();
    let (open, dismissed) = STATE.with(|state| {
        let state = state.borrow();
        (state.open, state.dismissed)
    });

    if open == Some(spec.id) && layer_window_visible(&label).unwrap_or(false) {
        log_info(&format!("[applet] cerrando {}", spec.id));
        dismiss_on_main(app, spec);
        return;
    }

    if closes_a_recent_dismissal(dismissed, spec.id, Instant::now()) {
        // El clic que lo cerró fue este mismo: ver `REOPEN_GUARD`.
        return;
    }

    open_on_main(app, spec, anchor.as_ref(), None);
}

fn open_on_main(
    app: &AppHandle,
    spec: &'static AppletSpec,
    anchor: Option<&AnchorRect>,
    size: Option<(f64, f64)>,
) {
    // Uno por vez: el anterior se va sin animación, porque el nuevo aparece en
    // el mismo instante y dos superficies fundiéndose a la vez se ven sucias.
    let previous = STATE.with(|state| state.borrow().open);
    if let Some(previous) = previous.filter(|previous| *previous != spec.id) {
        if let Some(previous) = applet_spec(previous) {
            hide_now(app, previous);
        }
    }

    let Some(primary) = get_primary_monitor(app) else {
        log_error("[applet] sin monitor primario: no hay dónde abrirlo");
        return;
    };
    // El panel vive en el primario, así que el applet también.
    let scale = primary.scale_factor();
    let monitor = (
        primary.size().width as f64 / scale,
        primary.size().height as f64 / scale,
    );
    let side = panel_position::read();
    let placement = place_applet(anchor, side, size.unwrap_or(spec.size), monitor);
    let label = spec.label();

    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.generation += 1;
        state.dismissed = None;
    });

    let shown = AppletShown {
        applet: spec.id,
        side: side.key(),
        origin: placement.origin,
    };

    if layer_window_exists(&label) {
        relocate_layer_window(
            &label,
            &Geometry {
                anchors: placement.anchors,
                size: placement.size,
                margins: placement.margins,
                exclusive_zone: Some(-1),
            },
        );
        if let Some(webview) = app.get_webview_window(&label) {
            let _ = webview.emit("applet-shown", shown);
        }
        show_layer_window(&label);
    } else if let Err(error) = spawn_applet(app, spec, &placement, &shown) {
        log_error(&format!("[applet] no se pudo abrir {}: {error}", spec.id));
        return;
    }

    log_info(&format!(
        "[applet] {} abierto en {:?}, {:?}",
        spec.id, placement.margins, placement.size
    ));

    STATE.with(|state| state.borrow_mut().open = Some(spec.id));
    announce(app, Some(spec.id));
}

fn spawn_applet(
    app: &AppHandle,
    spec: &'static AppletSpec,
    placement: &Placement,
    shown: &AppletShown,
) -> Result<(), Box<dyn std::error::Error>> {
    let primary = get_primary_monitor(app).ok_or("No primary monitor found")?;
    let gdk_monitor =
        find_gdk_monitor(&primary).ok_or("No GDK monitor matching the primary monitor")?;

    // La primera vez, el lado y el origen van en la ruta: el evento de
    // `applet-shown` saldría antes de que la página exista para oírlo.
    let route = format!(
        "index.html#/applets/{}?side={}&origin={}",
        spec.route, shown.side, shown.origin
    );

    let on_dismiss = {
        let app = app.clone();
        Rc::new(move || dismiss_on_main(&app, spec)) as Rc<dyn Fn()>
    };
    let on_hide = {
        let app = app.clone();
        Box::new(move || {
            let was_open = STATE.with(|state| {
                let mut state = state.borrow_mut();
                if state.open == Some(spec.id) {
                    state.open = None;
                    true
                } else {
                    false
                }
            });
            if was_open {
                announce(&app, None);
            }
        }) as Box<dyn Fn()>
    };

    spawn_layer_window(
        app,
        &spec.label(),
        &route,
        &gdk_monitor,
        placement.size,
        LayerSpec {
            namespace: "vasak-applet",
            // Encima del panel, como el centro de control: la sombra y el
            // borde no tienen que quedar debajo de la barra.
            layer: Layer::Overlay,
            anchors: placement.anchors,
            exclusive_zone: Some(-1),
            margins: placement.margins,
            keyboard: KeyboardMode::OnDemand,
            start_hidden: false,
            dismiss_on_unfocus: true,
            on_dismiss: Some(on_dismiss),
            on_hide: Some(on_hide),
        },
    )
}

/// Lo que recibe la página cuando la van a esconder.
#[derive(Debug, Clone, Serialize)]
pub struct AppletLeave {
    pub applet: &'static str,
    /// `true` cuando se esconde en el acto, sin tiempo para la salida: la
    /// página se deja invisible para que al volver a mostrarse no asome un
    /// cuadro viejo antes de la entrada.
    pub instant: bool,
}

fn tell_page_to_leave(app: &AppHandle, spec: &'static AppletSpec, instant: bool) {
    if let Some(webview) = app.get_webview_window(&spec.label()) {
        let _ = webview.emit(
            "applet-leave",
            AppletLeave {
                applet: spec.id,
                instant,
            },
        );
    }
}

/// Esconde el applet ya, sin animación.
fn hide_now(app: &AppHandle, spec: &'static AppletSpec) {
    STATE.with(|state| state.borrow_mut().generation += 1);
    tell_page_to_leave(app, spec, true);
    hide_layer_window(&spec.label());
}

/// Cierra el applet con su salida: le avisa a la página, que se desvanece, y
/// lo esconde cuando terminó.
fn dismiss_on_main(app: &AppHandle, spec: &'static AppletSpec) {
    let label = spec.label();
    if !layer_window_visible(&label).unwrap_or(false) {
        return;
    }

    let generation = STATE.with(|state| {
        let mut state = state.borrow_mut();
        // Ya se está cerrando: Escape y la pérdida de foco llegan juntos, y la
        // página también avisa por su lado.
        if matches!(state.dismissed, Some((id, _)) if id == spec.id) {
            return None;
        }
        state.dismissed = Some((spec.id, Instant::now()));
        state.generation += 1;
        Some(state.generation)
    });
    let Some(generation) = generation else {
        return;
    };

    tell_page_to_leave(app, spec, false);

    glib::timeout_add_local_once(LEAVE_DURATION, move || {
        let current = STATE.with(|state| state.borrow().generation);
        if current == generation {
            hide_layer_window(&label);
        }
    });
}

/// Cierra con su salida el applet `id`, si es el que está abierto. Es lo que
/// pide la página cuando termina lo que vino a hacer —un clic en el menú de la
/// bandeja, el botón de cerrar—.
///
/// Se nombra el applet y no «el que esté abierto»: el pedido de una página que
/// se estaba yendo puede llegar cuando ya se abrió otro, y cerraría ése.
pub fn dismiss_anchored_applet(app: &AppHandle, id: &str) -> Result<(), String> {
    let spec = applet_spec(id).ok_or_else(|| format!("no hay ningún applet «{id}»"))?;
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let open = STATE.with(|state| state.borrow().open);
        if open == Some(spec.id) {
            dismiss_on_main(&handle, spec);
        }
    })
    .map_err(|error| format!("no se pudo llegar al hilo principal: {error}"))
}

/// Cierra en el acto el applet abierto.
///
/// Para cuando el panel se muda de lado: el botón que lo abrió ya no está donde
/// estaba, y un applet colgando de la nada no se lee como parte del panel.
///
/// Sólo desde el hilo principal de GTK.
pub fn close_open_applet(app: &AppHandle) {
    let open = STATE.with(|state| state.borrow().open);
    if let Some(spec) = open.and_then(applet_spec) {
        log_info(&format!("[applet] {} cerrado de golpe", spec.id));
        hide_now(app, spec);
    }
}

/// Destruye las superficies de todos los applets.
///
/// Van atadas al monitor en que se crearon; cuando cambian los monitores ese
/// monitor puede no existir más. La próxima vez que se pidan se crean de nuevo
/// en el que corresponda.
///
/// Sólo desde el hilo principal de GTK.
pub fn destroy_applets(app: &AppHandle) {
    close_open_applet(app);
    destroy_layer_windows(app, &[LABEL_PREFIX]);
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONITOR: (f64, f64) = (1920.0, 1080.0);
    const APPLET: (f64, f64) = (400.0, 300.0);
    const AWAY: i32 = PANEL_THICKNESS + PANEL_GAP;
    const M: i32 = SCREEN_MARGIN;

    /// Un botón de 30 píxeles, con su centro en `center` a lo largo del panel.
    fn button(side: PanelPosition, center: f64) -> AnchorRect {
        if side.is_vertical() {
            AnchorRect {
                x: 4.0,
                y: center - 15.0,
                width: 30.0,
                height: 30.0,
            }
        } else {
            AnchorRect {
                x: center - 15.0,
                y: 4.0,
                width: 30.0,
                height: 30.0,
            }
        }
    }

    #[test]
    fn con_el_panel_arriba_el_applet_cuelga_debajo_del_boton() {
        let placement = place_applet(
            Some(&button(PanelPosition::Top, 900.0)),
            PanelPosition::Top,
            APPLET,
            MONITOR,
        );

        assert_eq!(placement.anchors, (true, false, true, false));
        // Centrado sobre el botón, y a 8 píxeles del borde de abajo del panel.
        assert_eq!(placement.margins, (700, 0, AWAY, 0));
        assert_eq!(placement.size, APPLET);
        assert_eq!(placement.origin, 200.0);
    }

    #[test]
    fn con_el_panel_abajo_el_applet_sube_desde_el_boton() {
        let placement = place_applet(
            Some(&button(PanelPosition::Bottom, 900.0)),
            PanelPosition::Bottom,
            APPLET,
            MONITOR,
        );

        assert_eq!(placement.anchors, (true, false, false, true));
        assert_eq!(placement.margins, (700, 0, 0, AWAY));
        assert_eq!(placement.origin, 200.0);
    }

    #[test]
    fn con_el_panel_a_la_izquierda_el_applet_sale_hacia_la_derecha() {
        let placement = place_applet(
            Some(&button(PanelPosition::Left, 500.0)),
            PanelPosition::Left,
            APPLET,
            MONITOR,
        );

        assert_eq!(placement.anchors, (true, false, true, false));
        // Alineado verticalmente con el botón.
        assert_eq!(placement.margins, (AWAY, 0, 350, 0));
        assert_eq!(placement.origin, 150.0);
    }

    #[test]
    fn con_el_panel_a_la_derecha_el_applet_sale_hacia_la_izquierda() {
        // El rectángulo es relativo al panel, y el panel de la derecha también
        // empieza arriba: la `y` no cambia.
        let placement = place_applet(
            Some(&button(PanelPosition::Right, 500.0)),
            PanelPosition::Right,
            APPLET,
            MONITOR,
        );

        assert_eq!(placement.anchors, (false, true, true, false));
        assert_eq!(placement.margins, (0, AWAY, 350, 0));
        assert_eq!(placement.origin, 150.0);
    }

    #[test]
    fn un_boton_pegado_al_borde_izquierdo_no_saca_el_applet_del_monitor() {
        let placement = place_applet(
            Some(&button(PanelPosition::Top, 20.0)),
            PanelPosition::Top,
            APPLET,
            MONITOR,
        );

        // Se corre hasta el margen del escritorio, sin despegarse del panel...
        assert_eq!(placement.margins, (M, 0, AWAY, 0));
        // ...y la animación sigue saliendo del botón, que ya no está al centro.
        assert_eq!(placement.origin, 20.0 - M as f64);
    }

    #[test]
    fn un_boton_pegado_al_borde_derecho_tampoco() {
        let placement = place_applet(
            Some(&button(PanelPosition::Bottom, 1905.0)),
            PanelPosition::Bottom,
            APPLET,
            MONITOR,
        );

        let left = 1920 - M - 400;
        assert_eq!(placement.margins, (left, 0, 0, AWAY));
        // El borde derecho del applet queda a un margen del monitor.
        assert_eq!(
            placement.margins.0 as f64 + placement.size.0,
            1920.0 - M as f64
        );
        assert_eq!(placement.origin, 1905.0 - left as f64);
    }

    #[test]
    fn a_los_costados_un_boton_pegado_arriba_o_abajo_tampoco() {
        let arriba = place_applet(
            Some(&button(PanelPosition::Left, 10.0)),
            PanelPosition::Left,
            APPLET,
            MONITOR,
        );
        assert_eq!(arriba.margins, (AWAY, 0, M, 0));
        assert_eq!(arriba.origin, 0.0);

        let abajo = place_applet(
            Some(&button(PanelPosition::Right, 1075.0)),
            PanelPosition::Right,
            APPLET,
            MONITOR,
        );
        assert_eq!(abajo.margins, (0, AWAY, 1080 - M - 300, 0));
        // El botón queda más allá del borde del applet: el origen se queda en
        // el borde, no afuera.
        assert_eq!(abajo.origin, 300.0);
    }

    #[test]
    fn un_applet_mas_grande_que_el_monitor_se_achica_para_entrar() {
        let chico = (800.0, 600.0);

        let arriba = place_applet(
            Some(&button(PanelPosition::Top, 400.0)),
            PanelPosition::Top,
            (1000.0, 700.0),
            chico,
        );
        assert_eq!(
            arriba.size,
            (800.0 - 2.0 * M as f64, 600.0 - (AWAY + M) as f64)
        );
        assert_eq!(arriba.margins, (M, 0, AWAY, 0));

        let costado = place_applet(
            Some(&button(PanelPosition::Right, 300.0)),
            PanelPosition::Right,
            (1000.0, 700.0),
            chico,
        );
        assert_eq!(
            costado.size,
            (800.0 - (AWAY + M) as f64, 600.0 - 2.0 * M as f64)
        );
        assert_eq!(costado.margins, (0, AWAY, M, 0));
    }

    #[test]
    fn nunca_sale_del_monitor_desde_ningun_boton_ni_ningun_lado() {
        for side in PanelPosition::ALL {
            let length = if side.is_vertical() {
                MONITOR.1
            } else {
                MONITOR.0
            };
            let mut center = 0.0;
            while center <= length {
                let placement = place_applet(Some(&button(side, center)), side, APPLET, MONITOR);
                let (left, right, top, bottom) = placement.margins;
                let (width, height) = placement.size;
                let x = if placement.anchors.0 {
                    left as f64
                } else {
                    MONITOR.0 - right as f64 - width
                };
                let y = if placement.anchors.2 {
                    top as f64
                } else {
                    MONITOR.1 - bottom as f64 - height
                };

                assert!(x >= 0.0 && x + width <= MONITOR.0, "{side:?} {center}");
                assert!(y >= 0.0 && y + height <= MONITOR.1, "{side:?} {center}");
                assert!(
                    (0.0..=if side.is_vertical() { height } else { width })
                        .contains(&placement.origin),
                    "{side:?} {center}"
                );
                center += 37.0;
            }
        }
    }

    #[test]
    fn nunca_tapa_el_panel() {
        // El applet cuelga del panel: su borde más cercano queda a 8 píxeles del
        // borde interno de la barra, en los cuatro lados.
        for side in PanelPosition::ALL {
            let placement = place_applet(Some(&button(side, 300.0)), side, APPLET, MONITOR);
            let (left, right, top, bottom) = placement.margins;
            let toward_panel = match side {
                PanelPosition::Top => top,
                PanelPosition::Bottom => bottom,
                PanelPosition::Left => left,
                PanelPosition::Right => right,
            };
            assert_eq!(toward_panel, PANEL_THICKNESS + PANEL_GAP, "{side:?}");
        }
    }

    #[test]
    fn sin_boton_va_centrado_en_el_eje_del_panel() {
        // Lo que abre un applet desde fuera del panel —el centro de control— no
        // tiene botón en la barra del que colgarlo.
        let placement = place_applet(None, PanelPosition::Top, APPLET, MONITOR);
        assert_eq!(placement.margins, (760, 0, AWAY, 0));
        assert_eq!(placement.origin, 200.0);
    }

    #[test]
    fn cada_applet_tiene_su_ruta_y_su_etiqueta_propias() {
        for spec in APPLETS {
            assert_eq!(applet_spec(spec.id), Some(spec));
            assert!(spec.label().starts_with(LABEL_PREFIX));
            assert!(spec.size.0 > 0.0 && spec.size.1 > 0.0, "{}", spec.id);
            let same_route = APPLETS.iter().filter(|other| other.route == spec.route);
            assert_eq!(same_route.count(), 1, "{}", spec.id);
        }
        assert_eq!(applet_spec("no-existe"), None);
    }

    /// Si una etiqueta de ventana entra en un patrón de la capability. Tauri
    /// acepta `*` como comodín; la capability usa sólo el del final
    /// (`desktop*`), y eso es lo que se reconoce.
    fn cubre(patron: &str, etiqueta: &str) -> bool {
        match patron.strip_suffix('*') {
            Some(prefijo) => etiqueta.starts_with(prefijo),
            None => patron == etiqueta,
        }
    }

    /// Cada applet tiene permisos.
    ///
    /// Una ventana que no está en `windows` de la capability no puede llamar a
    /// ningún comando: el tema se pide al plugin de configuración, así que el
    /// applet se abre sin tema y sin datos, y lo único que lo dice es un error
    /// en la consola del WebView. Pasó con la bandeja y con privacidad: al
    /// pasar a `applet_<id>` quedaron `applet_tray` y `applet_privacy` afuera, y
    /// la capability seguía nombrando `systray_popup`, que ya no existe.
    #[test]
    fn cada_applet_esta_en_la_capability() {
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../../capabilities/default.json"))
                .expect("capabilities/default.json se lee");
        let ventanas: Vec<&str> = capability["windows"]
            .as_array()
            .expect("la capability declara sus ventanas")
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(!ventanas.is_empty(), "la lista de ventanas vino vacía");

        let sin_permisos: Vec<String> = APPLETS
            .iter()
            .map(AppletSpec::label)
            .filter(|etiqueta| !ventanas.iter().any(|patron| cubre(patron, etiqueta)))
            .collect();
        assert!(
            sin_permisos.is_empty(),
            "estos applets no están en capabilities/default.json y se abren sin permisos \
             (sin tema, sin datos): {sin_permisos:?}"
        );
    }

    #[test]
    fn el_comodin_de_la_capability_se_lee_como_tauri() {
        assert!(cubre("desktop*", "desktop_1"));
        assert!(cubre("applet_tray", "applet_tray"));
        assert!(!cubre("applet_tray", "applet_tray2"));
        assert!(!cubre("desktop*", "panel"));
    }

    #[test]
    fn las_etiquetas_de_los_applets_no_chocan_con_las_del_shell() {
        // Rehacer el shell espera a que se liberen las etiquetas del panel, el
        // escritorio, el centro de control y el menú, por prefijo. Un applet
        // que empezara con uno de esos haría esperar a la reconstrucción por
        // algo que no tiene que ver.
        for spec in APPLETS {
            let label = spec.label();
            for shell in ["panel", "desktop", "control_center", "menu"] {
                assert!(!label.starts_with(shell), "{label}");
            }
        }
    }

    #[test]
    fn un_clic_justo_despues_de_perder_el_foco_cierra_y_no_reabre() {
        let now = Instant::now();
        let just_now = Some(("audio", now));

        // Tocar el botón del applet abierto le saca el foco antes de que llegue
        // el clic: ese clic es el que lo cerró.
        assert!(closes_a_recent_dismissal(
            just_now,
            "audio",
            now + Duration::from_millis(40)
        ));
        // El botón de otro applet sí abre el suyo.
        assert!(!closes_a_recent_dismissal(
            just_now,
            "network",
            now + Duration::from_millis(40)
        ));
        // Y pasado el rato, es un clic nuevo.
        assert!(!closes_a_recent_dismissal(
            just_now,
            "audio",
            now + REOPEN_GUARD + Duration::from_millis(1)
        ));
        assert!(!closes_a_recent_dismissal(None, "audio", now));
    }
}
