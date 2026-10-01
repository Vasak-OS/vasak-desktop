//! Pruebas de la bandeja contra **dobles de bus**: un `dbus-daemon` propio, con
//! su propia dirección, y elementos de mentira servidos desde la misma prueba.
//!
//! Nunca se toca el bus de la sesión: el panel instalado está escuchando ahí, y
//! una señal falsa lo confundiría. Si la máquina no tiene `dbus-daemon`, las
//! pruebas lo dicen y no hacen nada (no se dan por buenas en silencio: el
//! mensaje queda en la salida).

use crate::structs::{
    MenuDisposition, SystrayPopupPayload, SystrayPopupState, ToggleKind, ToggleState, TrayManager,
    TrayStatus,
};
use crate::tray::item_props::read_item;
use crate::tray::launcher_entry::{
    attach_badges, create_launcher_entry_store, record_signal, LAUNCHER_ENTRY_INTERFACE,
};
use crate::tray::pixmap::tests::{png_bytes, solid};
use crate::tray::pixmap::{Pixmap, MAX_ICON_SIDE};
use crate::tray::signals::{apply_item_signal, apply_menu_signal};
use crate::tray::sni_watcher::sni_proxy;
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime::RwLock;
use zbus::object_server::SignalContext;
use zbus::zvariant::{OwnedValue, Value};
use zbus::{fdo, interface, Connection, MatchRule, MessageStream, MessageType};

/// Un `dbus-daemon` de la prueba, hijo de este proceso. Se mata al soltarse.
///
/// Va sin `--fork`: con `--fork` había que esperar a que se cerrara la salida
/// del que lo lanza, y con varias pruebas a la vez esa espera se colgaba.
/// Así se lee la primera línea —la dirección— y el proceso queda en la mano.
struct PrivateBus {
    address: String,
    child: std::process::Child,
    dir: std::path::PathBuf,
}

impl PrivateBus {
    fn start() -> Option<Self> {
        use std::io::BufRead;
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("vasak-tray-bus-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).ok()?;
        let mut child = Command::new("dbus-daemon")
            .args([
                "--session",
                "--nofork",
                "--print-address=1",
                &format!("--address=unix:dir={}", dir.display()),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let mut line = String::new();
        let read = std::io::BufReader::new(child.stdout.take()?).read_line(&mut line);
        if read.is_err() || line.trim().is_empty() {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        Some(Self {
            address: line.trim().to_string(),
            child,
            dir,
        })
    }

    async fn connect(&self) -> Connection {
        self.connect_serving(Ok).await
    }

    async fn connect_serving<F>(&self, serve: F) -> Connection
    where
        F: FnOnce(
            zbus::connection::Builder<'static>,
        ) -> zbus::Result<zbus::connection::Builder<'static>>,
    {
        let builder = zbus::connection::Builder::address(self.address.as_str()).unwrap();
        let connect = serve(builder).unwrap().build();
        tokio::time::timeout(Duration::from_secs(5), connect)
            .await
            .expect("el bus privado no contestó")
            .expect("conexión al bus privado")
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

macro_rules! private_bus {
    () => {
        match PrivateBus::start() {
            Some(bus) => bus,
            None => {
                eprintln!("sin dbus-daemon en esta máquina: la prueba de bus no corre");
                return;
            }
        }
    };
}

/// Un StatusNotifierItem de mentira. Lo que está en `None` responde con error,
/// como Chromium con `IconName`.
#[derive(Default, Clone)]
struct FakeItem {
    id: Option<String>,
    title: Option<String>,
    status: Option<String>,
    icon_name: Option<String>,
    icon_pixmap: Option<Vec<Pixmap>>,
    overlay_icon_name: Option<String>,
    overlay_icon_pixmap: Option<Vec<Pixmap>>,
    attention_icon_name: Option<String>,
    attention_icon_pixmap: Option<Vec<Pixmap>>,
    attention_movie_name: Option<String>,
    tool_tip: Option<(String, Vec<Pixmap>, String, String)>,
    item_is_menu: Option<bool>,
}

fn missing<T>(value: &Option<T>) -> fdo::Result<T>
where
    T: Clone,
{
    value
        .clone()
        .ok_or_else(|| fdo::Error::UnknownProperty("no viene".into()))
}

#[interface(name = "org.kde.StatusNotifierItem")]
impl FakeItem {
    #[zbus(property)]
    fn id(&self) -> fdo::Result<String> {
        missing(&self.id)
    }
    #[zbus(property)]
    fn title(&self) -> fdo::Result<String> {
        missing(&self.title)
    }
    #[zbus(property)]
    fn status(&self) -> fdo::Result<String> {
        missing(&self.status)
    }
    #[zbus(property)]
    fn category(&self) -> String {
        "Communications".into()
    }
    /// Como Chromium: `i` y no `u`.
    #[zbus(property)]
    fn window_id(&self) -> i32 {
        0
    }
    #[zbus(property)]
    fn icon_name(&self) -> fdo::Result<String> {
        missing(&self.icon_name)
    }
    #[zbus(property)]
    fn icon_pixmap(&self) -> fdo::Result<Vec<Pixmap>> {
        missing(&self.icon_pixmap)
    }
    #[zbus(property)]
    fn overlay_icon_name(&self) -> fdo::Result<String> {
        missing(&self.overlay_icon_name)
    }
    #[zbus(property)]
    fn overlay_icon_pixmap(&self) -> fdo::Result<Vec<Pixmap>> {
        missing(&self.overlay_icon_pixmap)
    }
    #[zbus(property)]
    fn attention_icon_name(&self) -> fdo::Result<String> {
        missing(&self.attention_icon_name)
    }
    #[zbus(property)]
    fn attention_icon_pixmap(&self) -> fdo::Result<Vec<Pixmap>> {
        missing(&self.attention_icon_pixmap)
    }
    #[zbus(property)]
    fn attention_movie_name(&self) -> fdo::Result<String> {
        missing(&self.attention_movie_name)
    }
    #[zbus(property)]
    fn tool_tip(&self) -> fdo::Result<(String, Vec<Pixmap>, String, String)> {
        missing(&self.tool_tip)
    }
    #[zbus(property)]
    fn item_is_menu(&self) -> fdo::Result<bool> {
        missing(&self.item_is_menu)
    }

    #[zbus(signal)]
    async fn new_icon(ctxt: &SignalContext<'_>) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn new_overlay_icon(ctxt: &SignalContext<'_>) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn new_tool_tip(ctxt: &SignalContext<'_>) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn new_status(ctxt: &SignalContext<'_>, status: &str) -> zbus::Result<()>;
}

const ITEM_PATH: &str = "/StatusNotifierItem";
const MENU_PATH: &str = "/MenuBar";

/// El objeto se sirve **al armar** la conexión (`serve_at`) y no después con
/// `object_server().at()`: así, el despachador ya escucha cuando la conexión
/// aparece en el bus. Al revés, una llamada que llega en el primer instante se
/// pierde y la prueba se cuelga esperando la respuesta (pasaba una de cada
/// diez veces).
async fn serve_item(bus: &PrivateBus, item: FakeItem) -> Connection {
    bus.connect_serving(|builder| builder.serve_at(ITEM_PATH, item))
        .await
}

/// Espera a que el servidor conteste antes de hablarle.
///
/// Medido con `dbus-monitor` sobre el bus de la prueba: de vez en cuando
/// (una de cada setenta corridas) la primera llamada a una conexión recién
/// armada en este mismo proceso llega al servidor y no se contesta nunca, aun
/// con el objeto servido al armarla. Un `Ping` con reintentos absorbe ese
/// arranque; el elemento de verdad es otro proceso que ya estaba andando.
async fn wait_until_serving(client: &Connection, server: &Connection) {
    let name = unique(server);
    for _ in 0..40 {
        let ping = client.call_method(
            Some(name.as_str()),
            "/",
            Some("org.freedesktop.DBus.Peer"),
            "Ping",
            &(),
        );
        if let Ok(Ok(_)) = tokio::time::timeout(Duration::from_millis(250), ping).await {
            return;
        }
    }
    panic!("el servidor de la prueba no contesta");
}

fn unique(conn: &Connection) -> String {
    conn.unique_name().unwrap().to_string()
}

fn tray_manager_with(server: &Connection) -> TrayManager {
    let item = crate::structs::TrayItem {
        id: "fake".into(),
        service_name: unique(server),
        bus_name: Some(unique(server)),
        icon_name: Some("viejo".into()),
        icon_data: None,
        overlay_icon: None,
        attention_icon: None,
        attention_movie_name: None,
        title: None,
        tooltip: None,
        status: TrayStatus::Active,
        category: crate::structs::TrayCategory::ApplicationStatus,
        menu_path: Some(MENU_PATH.into()),
        item_is_menu: false,
        launcher: None,
        object_path: ITEM_PATH.into(),
        unique_name: Some(unique(server)),
        pid: Some(std::process::id()),
    };
    let mut map = HashMap::new();
    map.insert(unique(server), item);
    Arc::new(RwLock::new(map))
}

async fn stream_for(conn: &Connection, interface: &str) -> MessageStream {
    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface(interface)
        .unwrap()
        .build();
    MessageStream::for_match_rule(rule, conn, None)
        .await
        .unwrap()
}

async fn next_signal(stream: &mut MessageStream) -> zbus::Message {
    tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("la señal no llegó")
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn se_lee_todo_lo_que_el_elemento_manda() {
    let bus = private_bus!();
    let fake = FakeItem {
        id: Some("TelegramDesktop".into()),
        title: Some("Telegram".into()),
        status: Some("NeedsAttention".into()),
        icon_name: Some("telegram-panel".into()),
        icon_pixmap: Some(vec![
            solid(16, 16, [255, 0, 136, 204]),
            solid(48, 48, [255, 0, 136, 204]),
        ]),
        overlay_icon_name: Some("emblem-important".into()),
        overlay_icon_pixmap: Some(vec![]),
        attention_icon_name: Some("telegram-attention-panel".into()),
        attention_icon_pixmap: Some(vec![solid(22, 22, [255, 255, 0, 0])]),
        attention_movie_name: Some("telegram-blink".into()),
        tool_tip: Some((
            "telegram".into(),
            vec![],
            "Telegram".into(),
            "<b>3</b> mensajes sin leer".into(),
        )),
        item_is_menu: Some(true),
    };
    let server = serve_item(&bus, fake).await;
    let client = bus.connect().await;
    wait_until_serving(&client, &server).await;
    let name = unique(&server);
    let proxy = sni_proxy(&client, &name, ITEM_PATH).await.unwrap();
    let item = read_item(&proxy, &name, &name, ITEM_PATH).await;

    assert_eq!(item.id, "TelegramDesktop");
    assert_eq!(item.title.as_deref(), Some("Telegram"));
    assert_eq!(item.status, TrayStatus::NeedsAttention);
    assert_eq!(item.icon_name.as_deref(), Some("telegram-panel"));
    assert!(item.icon_data.is_some());
    let overlay = item.overlay_icon.unwrap();
    assert_eq!(
        (overlay.name.as_deref(), overlay.data),
        (Some("emblem-important"), None)
    );
    let attention = item.attention_icon.unwrap();
    assert!(attention.name.is_some() && attention.data.is_some());
    assert_eq!(item.attention_movie_name.as_deref(), Some("telegram-blink"));
    let tooltip = item.tooltip.unwrap();
    assert_eq!(tooltip.title.as_deref(), Some("Telegram"));
    assert_eq!(tooltip.description.as_deref(), Some("3 mensajes sin leer"));
    assert_eq!(tooltip.icon.unwrap().name.as_deref(), Some("telegram"));
    assert!(item.item_is_menu);
}

#[tokio::test]
async fn lo_que_no_viene_no_llega_a_la_vista() {
    let bus = private_bus!();
    // Lo que manda Chromium: casi todo vacío, y `IconName` con error.
    let fake = FakeItem {
        id: Some("discord_status_icon_1".into()),
        title: Some(String::new()),
        status: Some("Active".into()),
        icon_pixmap: Some(vec![solid(24, 24, [255, 88, 101, 242])]),
        overlay_icon_name: Some(String::new()),
        overlay_icon_pixmap: Some(vec![]),
        attention_icon_name: Some(String::new()),
        attention_icon_pixmap: Some(vec![]),
        attention_movie_name: Some(String::new()),
        tool_tip: Some((String::new(), vec![], "Discord".into(), String::new())),
        item_is_menu: Some(false),
        ..Default::default()
    };
    let server = serve_item(&bus, fake).await;
    let client = bus.connect().await;
    wait_until_serving(&client, &server).await;
    let name = unique(&server);
    let proxy = sni_proxy(&client, &name, ITEM_PATH).await.unwrap();
    let item = read_item(&proxy, &name, &name, ITEM_PATH).await;

    assert_eq!(item.icon_name, None);
    assert!(item.icon_data.is_some());
    assert_eq!(item.title, None);
    assert_eq!(item.overlay_icon, None);
    assert_eq!(item.attention_icon, None);
    assert_eq!(item.attention_movie_name, None);
    let tooltip = item.tooltip.unwrap();
    assert_eq!((tooltip.icon, tooltip.description), (None, None));

    // Y uno que no responde nada: sigue siendo un elemento, sin inventos.
    let server = serve_item(&bus, FakeItem::default()).await;
    wait_until_serving(&client, &server).await;
    let name = unique(&server);
    let proxy = sni_proxy(&client, &name, ITEM_PATH).await.unwrap();
    let item = read_item(&proxy, &name, &name, ITEM_PATH).await;
    assert_eq!(item.id, name);
    assert_eq!(item.status, TrayStatus::Passive);
    assert!(item.icon_name.is_none() && item.icon_data.is_none() && item.tooltip.is_none());
    assert!(!item.item_is_menu);
}

#[tokio::test]
async fn las_senales_releen_lo_que_cambio() {
    let bus = private_bus!();
    let fake = FakeItem {
        icon_name: Some("viejo".into()),
        status: Some("Active".into()),
        ..Default::default()
    };
    let server = serve_item(&bus, fake).await;
    let client = bus.connect().await;
    wait_until_serving(&client, &server).await;
    let manager = tray_manager_with(&server);
    let key = unique(&server);
    let mut stream = stream_for(&client, "org.kde.StatusNotifierItem").await;

    // Cambia el icono y avisa.
    let iface = server
        .object_server()
        .interface::<_, FakeItem>(ITEM_PATH)
        .await
        .unwrap();
    iface.get_mut().await.icon_name = Some("nuevo".into());
    FakeItem::new_icon(iface.signal_context()).await.unwrap();
    let message = next_signal(&mut stream).await;
    assert!(apply_item_signal(&client, &manager, &message).await);
    assert_eq!(
        manager.read().await[&key].icon_name.as_deref(),
        Some("nuevo")
    );

    // Aparece la insignia.
    iface.get_mut().await.overlay_icon_name = Some("emblem-new".into());
    FakeItem::new_overlay_icon(iface.signal_context())
        .await
        .unwrap();
    let message = next_signal(&mut stream).await;
    assert!(apply_item_signal(&client, &manager, &message).await);
    let overlay = manager.read().await[&key].overlay_icon.clone().unwrap();
    assert_eq!(overlay.name.as_deref(), Some("emblem-new"));

    // El globo.
    iface.get_mut().await.tool_tip =
        Some((String::new(), vec![], "Título".into(), "Detalle".into()));
    FakeItem::new_tool_tip(iface.signal_context())
        .await
        .unwrap();
    let message = next_signal(&mut stream).await;
    assert!(apply_item_signal(&client, &manager, &message).await);
    let tooltip = manager.read().await[&key].tooltip.clone().unwrap();
    assert_eq!(tooltip.description.as_deref(), Some("Detalle"));

    // `NewStatus` trae el estado: se toma del argumento, sin leer la propiedad
    // (que acá sigue diciendo «Active»).
    FakeItem::new_status(iface.signal_context(), "NeedsAttention")
        .await
        .unwrap();
    let message = next_signal(&mut stream).await;
    assert!(apply_item_signal(&client, &manager, &message).await);
    assert_eq!(
        manager.read().await[&key].status,
        TrayStatus::NeedsAttention
    );

    // Una señal de otro emisor no le toca a nadie.
    let other = serve_item(&bus, FakeItem::default()).await;
    wait_until_serving(&client, &other).await;
    let iface = other
        .object_server()
        .interface::<_, FakeItem>(ITEM_PATH)
        .await
        .unwrap();
    FakeItem::new_icon(iface.signal_context()).await.unwrap();
    let message = next_signal(&mut stream).await;
    assert!(!apply_item_signal(&client, &manager, &message).await);
}

/// `(ia{sv}av)`: una entrada de `GetLayout` con sus hijos.
type LayoutNode = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

/// Un menú de mentira con todo lo que puede mandar una entrada.
struct FakeMenu {
    revision: u32,
}

fn menu_entry(id: i32, props: Vec<(&str, Value<'static>)>) -> Value<'static> {
    let map: HashMap<String, Value> = props.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    Value::from((id, map, Vec::<Value>::new()))
}

#[interface(name = "com.canonical.dbusmenu")]
impl FakeMenu {
    fn get_layout(&self, _parent: i32, _depth: i32, _props: Vec<String>) -> (u32, LayoutNode) {
        let png = Value::Array(zbus::zvariant::Array::from(png_bytes(16, 16)));
        let broken = Value::Array(zbus::zvariant::Array::from(
            b"\x89PNG\r\n\x1a\nroto".to_vec(),
        ));
        let huge = Value::Array(zbus::zvariant::Array::from(png_bytes(400, 400)));
        let children: Vec<OwnedValue> = vec![
            menu_entry(
                1,
                vec![("label", Value::from("_Abrir")), ("icon-data", png)],
            ),
            menu_entry(
                2,
                vec![("label", Value::from("Roto")), ("icon-data", broken)],
            ),
            menu_entry(
                3,
                vec![("label", Value::from("Grande")), ("icon-data", huge)],
            ),
            menu_entry(
                4,
                vec![
                    ("label", Value::from("Silenciar")),
                    ("toggle-type", Value::from("checkmark")),
                    ("toggle-state", Value::from(-1i32)),
                ],
            ),
            menu_entry(
                5,
                vec![
                    ("label", Value::from("Ocupado")),
                    ("toggle-type", Value::from("radio")),
                    ("toggle-state", Value::from(1i32)),
                ],
            ),
            menu_entry(6, vec![("type", Value::from("separator"))]),
            menu_entry(
                7,
                vec![
                    ("label", Value::from("Salir")),
                    ("shortcut", Value::from(vec![vec!["Control", "q"]])),
                    ("disposition", Value::from("alert")),
                ],
            ),
        ]
        .into_iter()
        .map(|v| OwnedValue::try_from(v).unwrap())
        .collect();
        (self.revision, (0, HashMap::new(), children))
    }

    fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    #[zbus(signal)]
    async fn items_properties_updated(
        ctxt: &SignalContext<'_>,
        updated: Vec<(i32, HashMap<String, Value<'_>>)>,
        removed: Vec<(i32, Vec<String>)>,
    ) -> zbus::Result<()>;
}

#[tokio::test]
async fn el_menu_trae_casillas_iconos_atajos_y_disposicion() {
    let bus = private_bus!();
    let server = bus.connect().await;
    server
        .object_server()
        .at(MENU_PATH, FakeMenu { revision: 1 })
        .await
        .unwrap();
    server
        .object_server()
        .at(ITEM_PATH, FakeItem::default())
        .await
        .unwrap();
    let client = bus.connect().await;
    let name = unique(&server);

    // Con reintento: ver `wait_until_serving`. `GetLayout` tiene tope, así que
    // una llamada perdida vuelve como error en vez de colgar la prueba.
    let mut items = Vec::new();
    for _ in 0..3 {
        if let Ok(read) = crate::commands::load_dbus_menu_level(&client, &name, MENU_PATH, 0).await
        {
            items = read;
            break;
        }
    }
    assert!(!items.is_empty(), "el menú de la prueba no se pudo leer");
    let by_id = |id: i32| items.iter().find(|e| e.id == id).unwrap().clone();

    assert_eq!(by_id(1).label, "Abrir");
    assert!(by_id(1).icon.unwrap().data.is_some());
    // El PNG roto no deja icono.
    assert_eq!(by_id(2).icon, None);
    // El enorme llega achicado.
    let big = by_id(3).icon.unwrap().data.unwrap();
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, big).unwrap();
    assert_eq!(
        image::load_from_memory(&bytes).unwrap().width(),
        MAX_ICON_SIDE
    );
    // −1 es indeterminado.
    let mute = by_id(4).toggle.unwrap();
    assert_eq!(
        (mute.kind, mute.state),
        (ToggleKind::Checkmark, ToggleState::Indeterminate)
    );
    let busy = by_id(5).toggle.unwrap();
    assert_eq!(
        (busy.kind, busy.state),
        (ToggleKind::Radio, ToggleState::On)
    );
    assert_eq!(by_id(6).menu_type, "separator");
    let quit = by_id(7);
    assert_eq!(
        quit.shortcut,
        Some(vec![vec!["Control".to_string(), "q".to_string()]])
    );
    assert_eq!(quit.disposition, Some(MenuDisposition::Alert));

    // Con el menú abierto, `ItemsPropertiesUpdated` cambia la entrada sin
    // volver a pedir el menú.
    let manager = tray_manager_with(&server);
    let popup = SystrayPopupState(std::sync::Mutex::new(Some(SystrayPopupPayload {
        icon_id: String::new(),
        icon_data: None,
        tooltip: None,
        status: None,
        title: String::new(),
        service_name: name.clone(),
        items: items.clone(),
    })));
    let mut stream = stream_for(&client, "com.canonical.dbusmenu").await;
    let iface = server
        .object_server()
        .interface::<_, FakeMenu>(MENU_PATH)
        .await
        .unwrap();
    let mut props = HashMap::new();
    props.insert("toggle-state".to_string(), Value::from(1i32));
    FakeMenu::items_properties_updated(
        iface.signal_context(),
        vec![(4, props)],
        vec![(7, vec!["disposition".into()])],
    )
    .await
    .unwrap();
    let message = next_signal(&mut stream).await;
    assert!(apply_menu_signal(&client, &manager, &popup, &message).await);
    let state = popup.0.lock().unwrap();
    let after = &state.as_ref().unwrap().items;
    let mute = after.iter().find(|e| e.id == 4).unwrap();
    assert_eq!(mute.toggle.unwrap().state, ToggleState::On);
    assert_eq!(after.iter().find(|e| e.id == 7).unwrap().disposition, None);
}

async fn emit_update(conn: &Connection, uri: &str, props: Vec<(&str, Value<'_>)>) {
    let map: HashMap<&str, Value> = props.into_iter().collect();
    conn.emit_signal(
        None::<&str>,
        "/com/canonical/unity/launcherentry/1",
        LAUNCHER_ENTRY_INTERFACE,
        "Update",
        &(uri, map),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn launcher_entry_se_asocia_al_elemento_del_mismo_proceso() {
    let bus = private_bus!();
    let app = bus.connect().await;
    let panel = bus.connect().await;
    let store = create_launcher_entry_store();
    let mut stream = stream_for(&panel, LAUNCHER_ENTRY_INTERFACE).await;

    // Progreso fuera de rango y un contador: se recorta y se dibuja.
    emit_update(
        &app,
        "application://org.telegram.desktop.desktop",
        vec![
            ("count", Value::I64(3)),
            ("count-visible", Value::Bool(true)),
            ("progress", Value::F64(1.4)),
            ("progress-visible", Value::Bool(true)),
        ],
    )
    .await;
    let message = next_signal(&mut stream).await;
    assert!(record_signal(&panel, &store, &message).await);
    let entries = store.read().await.clone();
    let entry = &entries["application://org.telegram.desktop.desktop"];
    // Las dos conexiones son de este proceso: el pid lo dice el bus.
    assert_eq!(entry.pid, Some(std::process::id()));

    // El elemento de la bandeja de este mismo proceso, con otro nombre.
    let server = serve_item(&bus, FakeItem::default()).await;
    wait_until_serving(&panel, &server).await;
    let manager = tray_manager_with(&server);
    {
        let mut items = manager.write().await;
        assert!(attach_badges(&mut items, &entries));
        let badge = items.values().next().unwrap().launcher.clone().unwrap();
        assert_eq!((badge.count, badge.progress), (Some(3), Some(1.0)));
    }

    // Una entrada de otra aplicación sin elemento en la bandeja: se guarda y no
    // le cae a nadie.
    let mut lonely = HashMap::new();
    lonely.insert(
        "application://firefox.desktop".to_string(),
        crate::tray::launcher_entry::LauncherEntry {
            sender: ":1.999".into(),
            pid: Some(1),
            state: crate::tray::launcher_entry::LauncherEntryState {
                urgent: true,
                ..Default::default()
            },
        },
    );
    let mut items = manager.write().await;
    attach_badges(&mut items, &lonely);
    assert_eq!(items.values().next().unwrap().launcher, None);

    // Y una firma equivocada no rompe nada.
    drop(items);
    app.emit_signal(
        None::<&str>,
        "/x",
        LAUNCHER_ENTRY_INTERFACE,
        "Update",
        &("solo un texto",),
    )
    .await
    .unwrap();
    let message = next_signal(&mut stream).await;
    assert!(!record_signal(&panel, &store, &message).await);
}
