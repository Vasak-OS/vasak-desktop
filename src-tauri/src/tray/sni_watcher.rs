use super::{emit_tray_update, TrayManager};
use crate::dbus_pool::DbusPool;
use crate::logger::{log_debug, log_error, log_info, log_warning};
use crate::tray::item_props::read_item;
use crate::tray::launcher_entry::{attach_badges, connection_pid, unique_name_of, LauncherEntryStore};
use crate::tray::sni_item::SniItemProxy;
use futures_util::stream::StreamExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use zbus::fdo::RequestNameFlags;
use zbus::message::Header;
use zbus::object_server::SignalContext;
use zbus::{fdo, interface, Connection, MatchRule, MessageStream, MessageType};

const SNI_WATCHER_SERVICE: &str = "org.kde.StatusNotifierWatcher";
const SNI_WATCHER_PATH: &str = "/StatusNotifierWatcher";

/// Interval for periodic reconciliation of tray items against active D-Bus names.
const RECONCILIATION_INTERVAL: Duration = Duration::from_secs(30);

/// Timeout for the ListNames D-Bus call during reconciliation.
const LIST_NAMES_TIMEOUT: Duration = Duration::from_secs(5);

/// Only one watcher may run per process: the tray applet starts one at boot and
/// the panel webview calls `init_sni_watcher` again on every reload. A second
/// watcher would duplicate every stream, reconciliation loop and bus name request.
static WATCHER_STARTED: AtomicBool = AtomicBool::new(false);

/// Normalises the argument of `RegisterStatusNotifierItem`. The spec lets a client
/// pass either its bus name (`:1.42`) or the object path of the item, in which case
/// the service is the caller's own bus name. Both forms are stored as
/// `<bus name><object path>`, which is what `commands::tray` splits back into a
/// destination and a path; a bare bus name is kept verbatim because the default
/// `/StatusNotifierItem` path is implied there.
fn normalise_service(service: &str, sender: Option<&str>) -> Option<String> {
    if service.starts_with('/') {
        sender.map(|sender| format!("{}{}", sender, service))
    } else {
        Some(service.to_string())
    }
}

/// Los nombres conocidos con los que se registran los elementos.
fn is_item_bus_name(name: &str) -> bool {
    name.starts_with("org.kde.StatusNotifierItem-")
        || name.starts_with("org.freedesktop.StatusNotifierItem-")
}

/// Un proxy del elemento **sin caché de propiedades**. El elemento no emite
/// `PropertiesChanged` —avisa con `NewIcon` y compañía—, así que un proxy con
/// caché contestaría para siempre lo primero que leyó.
pub async fn sni_proxy<'a>(
    connection: &Connection,
    bus_name: &'a str,
    object_path: &'a str,
) -> zbus::Result<SniItemProxy<'a>> {
    SniItemProxy::builder(connection)
        .destination(bus_name)?
        .path(object_path)?
        .cache_properties(zbus::CacheProperties::No)
        .build()
        .await
}

/// True when `id` (a normalised item identifier) belongs to `bus_name`.
fn is_owned_by(id: &str, bus_name: &str) -> bool {
    id == bus_name || id.starts_with(&format!("{}/", bus_name))
}

/// The `org.kde.StatusNotifierWatcher` service exported at [`SNI_WATCHER_PATH`].
///
/// Registration handlers must reply immediately: a tray client blocks on the call
/// and gives up on its D-Bus timeout, so reading the item properties (several
/// round trips to the client itself) is deferred to a spawned task.
struct StatusNotifierWatcher {
    connection: Connection,
    tray_manager: TrayManager,
    app_handle: AppHandle,
    items: Vec<String>,
    hosts: Vec<String>,
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl StatusNotifierWatcher {
    async fn register_status_notifier_item(
        &mut self,
        service: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
    ) -> fdo::Result<()> {
        let sender = header.sender().map(|sender| sender.as_str());
        let id = normalise_service(service, sender).ok_or_else(|| {
            fdo::Error::InvalidArgs(format!(
                "RegisterStatusNotifierItem con object path '{}' y sin sender",
                service
            ))
        })?;

        log_info(&format!("[SNI] Registrando item: {}", id));

        if !self.items.iter().any(|known| known == &id) {
            self.items.push(id.clone());
            StatusNotifierWatcher::status_notifier_item_registered(&ctxt, &id).await?;
            self.registered_status_notifier_items_changed(&ctxt).await?;
        }

        // Fetching the item properties needs the caller to answer us in turn, so it
        // cannot happen before this method returns. It is spawned on Tauri's runtime
        // because zbus dispatches this call on its own executor thread, where
        // `tokio::spawn` would panic for lack of a runtime context.
        let connection = self.connection.clone();
        let tray_manager = self.tray_manager.clone();
        let app_handle = self.app_handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) =
                SniWatcher::register_item(&connection, &tray_manager, &app_handle, &id).await
            {
                log_error(&format!("[SNI] Error registrando item {}: {}", id, e));
            }
        });

        Ok(())
    }

    async fn register_status_notifier_host(
        &mut self,
        service: String,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
    ) -> fdo::Result<()> {
        log_info(&format!("[SNI] Registrando host: {}", service));

        if !self.hosts.iter().any(|known| known == &service) {
            self.hosts.push(service);
        }

        // IsStatusNotifierHostRegistered never flips, so only the signal is emitted.
        StatusNotifierWatcher::status_notifier_host_registered(&ctxt).await?;
        Ok(())
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.clone()
    }

    /// The panel is itself the host, so this is true from the moment the interface
    /// is served. Clients that gate their icon on this property would otherwise
    /// never show up.
    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        ctxt: &SignalContext<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        ctxt: &SignalContext<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(ctxt: &SignalContext<'_>) -> zbus::Result<()>;
}

/// Adds `id` to the watcher's registered list and announces it, unless it is
/// already known. Used for items discovered on the bus rather than registered
/// through the interface.
async fn remember_item(connection: &Connection, id: &str) {
    let object_server = connection.object_server();
    let iface_ref = match object_server
        .interface::<_, StatusNotifierWatcher>(SNI_WATCHER_PATH)
        .await
    {
        Ok(iface_ref) => iface_ref,
        Err(e) => {
            log_debug(&format!("[SNI] Interfaz del watcher no disponible: {}", e));
            return;
        }
    };

    let ctxt = iface_ref.signal_context();
    {
        let mut iface = iface_ref.get_mut().await;
        if iface.items.iter().any(|known| known == id) {
            return;
        }
        iface.items.push(id.to_string());
        if let Err(e) = iface.registered_status_notifier_items_changed(ctxt).await {
            log_debug(&format!(
                "[SNI] Error notificando cambio de RegisteredStatusNotifierItems: {}",
                e
            ));
        }
    }

    if let Err(e) = StatusNotifierWatcher::status_notifier_item_registered(ctxt, id).await {
        log_debug(&format!(
            "[SNI] Error emitiendo StatusNotifierItemRegistered para {}: {}",
            id, e
        ));
    }
}

/// Drops every registered identifier matching `matches` from the watcher and tells
/// clients about it. Takes the write lock only when there is something to remove,
/// since this runs on every name that disappears from the bus.
async fn forget_items<F>(connection: &Connection, matches: F)
where
    F: Fn(&str) -> bool,
{
    let object_server = connection.object_server();
    let iface_ref = match object_server
        .interface::<_, StatusNotifierWatcher>(SNI_WATCHER_PATH)
        .await
    {
        Ok(iface_ref) => iface_ref,
        Err(e) => {
            log_debug(&format!("[SNI] Interfaz del watcher no disponible: {}", e));
            return;
        }
    };

    let gone: Vec<String> = {
        let iface = iface_ref.get().await;
        iface
            .items
            .iter()
            .filter(|id| matches(id.as_str()))
            .cloned()
            .collect()
    };

    if gone.is_empty() {
        return;
    }

    let ctxt = iface_ref.signal_context();
    {
        let mut iface = iface_ref.get_mut().await;
        iface.items.retain(|id| !gone.contains(id));
        if let Err(e) = iface.registered_status_notifier_items_changed(ctxt).await {
            log_debug(&format!(
                "[SNI] Error notificando cambio de RegisteredStatusNotifierItems: {}",
                e
            ));
        }
    }

    for id in gone {
        if let Err(e) = StatusNotifierWatcher::status_notifier_item_unregistered(ctxt, &id).await {
            log_debug(&format!(
                "[SNI] Error emitiendo StatusNotifierItemUnregistered para {}: {}",
                id, e
            ));
        }
    }
}

pub struct SniWatcher {
    connection: Connection,
    tray_manager: TrayManager,
    app_handle: AppHandle,
}

impl SniWatcher {
    /// Starts the watcher unless this process already runs one, in which case it is
    /// a no-op so both the applet and the `init_sni_watcher` command can call it.
    pub async fn ensure_started(
        tray_manager: TrayManager,
        app_handle: AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if WATCHER_STARTED.swap(true, Ordering::SeqCst) {
            log_debug("[SNI] Watcher ya iniciado en este proceso, omitiendo");
            return Ok(());
        }

        // Release the slot on failure so a later call can try again.
        let watcher = match Self::new(tray_manager, app_handle).await {
            Ok(watcher) => watcher,
            Err(e) => {
                WATCHER_STARTED.store(false, Ordering::SeqCst);
                return Err(e);
            }
        };

        let result = watcher.start_watching().await;
        if result.is_err() {
            WATCHER_STARTED.store(false, Ordering::SeqCst);
        }

        result
    }

    async fn new(
        tray_manager: TrayManager,
        app_handle: AppHandle,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Use shared session bus from DbusPool if available, otherwise create own connection
        let connection = if let Some(pool) = app_handle.try_state::<DbusPool>() {
            match pool.session().await {
                Some(conn) => conn,
                None => Connection::session().await?,
            }
        } else {
            Connection::session().await?
        };

        Ok(Self {
            connection,
            tray_manager,
            app_handle,
        })
    }

    async fn start_watching(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Serve the interface before taking the name: clients watch for the name to
        // appear and call straight away, and would otherwise hit an unserved path.
        self.connection
            .object_server()
            .at(
                SNI_WATCHER_PATH,
                StatusNotifierWatcher {
                    connection: self.connection.clone(),
                    tray_manager: self.tray_manager.clone(),
                    app_handle: self.app_handle.clone(),
                    items: Vec::new(),
                    hosts: Vec::new(),
                },
            )
            .await?;

        // DoNotQueue without ReplaceExisting: a watcher already on the bus keeps its
        // items and its clients, and we do not want to steal them halfway.
        match self
            .connection
            .request_name_with_flags(SNI_WATCHER_SERVICE, RequestNameFlags::DoNotQueue.into())
            .await
        {
            Ok(_) => log_info("[SNI] Nombre org.kde.StatusNotifierWatcher adquirido"),
            Err(zbus::Error::NameTaken) => log_warning(
                "[SNI] Otro StatusNotifierWatcher posee el nombre, no se reemplaza",
            ),
            Err(e) => return Err(e.into()),
        }

        // Watch for services leaving the bus so their items do not linger.
        let name_owner_rule = MatchRule::builder()
            .msg_type(MessageType::Signal)
            .interface("org.freedesktop.DBus")?
            .member("NameOwnerChanged")?
            .build();

        let mut name_stream =
            MessageStream::for_match_rule(name_owner_rule, &self.connection, None).await?;

        tokio::spawn({
            let tray_manager = self.tray_manager.clone();
            let app_handle = self.app_handle.clone();
            let connection = self.connection.clone();

            async move {
                while let Some(msg) = name_stream.next().await {
                    let Ok(message) = msg else { continue };
                    // The body must outlive the borrowed names deserialised from it.
                    let body = message.body();
                    let Ok((name, _old_owner, new_owner)) =
                        body.deserialize::<(&str, &str, &str)>()
                    else {
                        continue;
                    };

                    if !new_owner.is_empty() {
                        continue;
                    }

                    // Check if the disconnected name is tracked directly or via bus_name
                    let is_tracked = {
                        let manager = tray_manager.read().await;
                        manager.contains_key(name)
                            || manager.values().any(|v| v.bus_name.as_deref() == Some(name))
                    };
                    if is_tracked {
                        log_debug(&format!("[SNI] Name owner changed, removing: {}", name));
                        Self::unregister_item(&tray_manager, &app_handle, name).await;
                    }

                    forget_items(&connection, |id| is_owned_by(id, name)).await;
                }
            }
        });

        // Lo que cambia después de registrarse: iconos, globo, estado y menú.
        if let Err(e) = crate::tray::signals::start(
            self.connection.clone(),
            self.tray_manager.clone(),
            self.app_handle.clone(),
        )
        .await
        {
            log_error(&format!("[SNI] No se pudieron escuchar las señales de los elementos: {e}"));
        }

        // Progreso, contador y urgencia de las aplicaciones.
        if let Some(store) = self.app_handle.try_state::<LauncherEntryStore>() {
            if let Err(e) = crate::tray::launcher_entry::start(
                self.connection.clone(),
                self.tray_manager.clone(),
                (*store).clone(),
                self.app_handle.clone(),
            )
            .await
            {
                log_error(&format!("[LauncherEntry] No se pudo escuchar Update: {e}"));
            }
        }

        // Spawn periodic reconciliation task (every 30s)
        self.start_periodic_reconciliation();

        // Discover existing StatusNotifierItems
        self.discover_existing_items().await?;

        Ok(())
    }

    /// Spawns a background task that periodically reconciles tray items against
    /// active D-Bus names. Removes stale entries whose bus_name no longer has
    /// an active D-Bus owner. On ListNames failure/timeout (5s), skips the cycle.
    fn start_periodic_reconciliation(&self) {
        let tray_manager = self.tray_manager.clone();
        let app_handle = self.app_handle.clone();
        let connection = self.connection.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(RECONCILIATION_INTERVAL);
            // The first tick completes immediately; skip it since we just discovered items
            interval.tick().await;

            loop {
                interval.tick().await;

                // Call ListNames with a 5s timeout
                let active_names = match tokio::time::timeout(
                    LIST_NAMES_TIMEOUT,
                    Self::call_list_names(&connection),
                )
                .await
                {
                    Ok(Ok(names)) => names,
                    Ok(Err(e)) => {
                        log_debug(&format!(
                            "[SNI] Reconciliation: ListNames failed: {}. Skipping cycle.",
                            e
                        ));
                        continue;
                    }
                    Err(_) => {
                        log_debug("[SNI] Reconciliation: ListNames timed out (5s). Skipping cycle.");
                        continue;
                    }
                };

                // Compare stored items against active names and prune stale entries
                let removed = {
                    let mut manager = tray_manager.write().await;
                    let mut removed: Vec<String> = Vec::new();

                    manager.retain(|key, item| {
                        // Check if the item's bus_name or the map key is still active
                        let bus_name_active = item
                            .bus_name
                            .as_ref()
                            .map(|bn| active_names.contains(bn))
                            .unwrap_or(true); // If no bus_name, keep it (can't verify)

                        let key_active = active_names.contains(key);

                        // Keep if either the key or bus_name is still active
                        let keep = key_active || bus_name_active;
                        if !keep {
                            log_info(&format!(
                                "[SNI] Reconciliation: pruning stale tray item: {} (bus_name: {:?})",
                                key,
                                item.bus_name
                            ));
                            removed.push(key.clone());
                        }
                        keep
                    });

                    removed
                };

                // Emit tray-update if any items were removed
                if !removed.is_empty() {
                    forget_items(&connection, |id| removed.iter().any(|gone| gone == id)).await;
                    emit_tray_update(&app_handle).await;
                }
            }
        });
    }

    /// Calls org.freedesktop.DBus.ListNames() and returns the list of active bus names.
    async fn call_list_names(connection: &Connection) -> Result<Vec<String>, zbus::Error> {
        let proxy = zbus::Proxy::new(
            connection,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .await?;

        let names: Vec<String> = proxy.call("ListNames", &()).await?;
        Ok(names)
    }

    /// Reads the item properties and publishes it. `service_name` must already be
    /// normalised by [`normalise_service`].
    async fn register_item(
        connection: &Connection,
        tray_manager: &TrayManager,
        app_handle: &AppHandle,
        service_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (bus_name, object_path) = if service_name.contains('/') {
            let parts: Vec<&str> = service_name.splitn(2, '/').collect();
            (parts[0], format!("/{}", parts[1]))
        } else {
            (service_name, "/StatusNotifierItem".to_string())
        };

        let proxy = sni_proxy(connection, bus_name, &object_path).await?;

        let mut item = read_item(&proxy, service_name, bus_name, &object_path).await;
        item.menu_path = Self::read_menu_path(&proxy, &item.id).await;
        item.unique_name = unique_name_of(connection, bus_name).await;
        item.pid = connection_pid(connection, bus_name).await;

        {
            let mut manager = tray_manager.write().await;
            manager.insert(service_name.to_string(), item);
        }

        // Si la aplicación ya había publicado progreso o contador, le toca ahora.
        if let Some(store) = app_handle.try_state::<LauncherEntryStore>() {
            let entries = store.read().await;
            let mut manager = tray_manager.write().await;
            attach_badges(&mut manager, &entries);
        }

        emit_tray_update(app_handle).await;
        Ok(())
    }

    async fn unregister_item(
        tray_manager: &TrayManager,
        app_handle: &AppHandle,
        name: &str,
    ) {
        log_info(&format!("[SNI] Desregistrando item: {}", name));

        {
            let mut manager = tray_manager.write().await;
            if manager.contains_key(name) {
                manager.remove(name);
            } else {
                manager.retain(|_, v| v.bus_name.as_deref() != Some(name));
            }
        }

        emit_tray_update(app_handle).await;
    }

    /// El camino del menú. El que publica el elemento gana; el de libayatana es
    /// una suposición que sólo acierta con las aplicaciones hechas sobre
    /// libayatana-appindicator, así que ir a buscarlo cuando el elemento sí dijo
    /// dónde vive su menú es como quedaban elementos sin menú contextual.
    async fn read_menu_path(proxy: &SniItemProxy<'_>, id: &str) -> Option<String> {
        match proxy
            .menu()
            .await
            .ok()
            .map(|path| path.as_str().to_string())
            .filter(|path| !path.is_empty() && path != "/")
        {
            Some(path) => Some(path),
            None => {
                log_info(&format!(
                    "[SNI] {id} no publica Menu; se prueba la ruta de libayatana"
                ));
                Some(format!("/org/ayatana/NotificationItem/{}/Menu", id))
            }
        }
    }

    async fn discover_existing_items(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Query existing StatusNotifierItems
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .await?;

        let names: Vec<String> = proxy.call("ListNames", &()).await?;

        for name in names {
            // La especificación de freedesktop nombra los elementos
            // `org.freedesktop.StatusNotifierItem-PID-ID` y la de KDE
            // `org.kde.StatusNotifierItem-…`. Sólo se buscaba la segunda, y
            // Chromium, Electron y Qt usan la primera: los que ya estaban
            // abiertos al arrancar el panel no aparecían hasta re-registrarse.
            if is_item_bus_name(&name) {
                // The error is flattened to a String right away: `Box<dyn Error>` is
                // not `Send` and would poison this future for the applet runtime.
                let registered = Self::register_item(
                    &self.connection,
                    &self.tray_manager,
                    &self.app_handle,
                    &name,
                )
                .await
                .map_err(|e| e.to_string());

                match registered {
                    Ok(()) => remember_item(&self.connection, &name).await,
                    Err(e) => log_error(&format!(
                        "[SNI] Error registrando item existente {}: {}",
                        name, e
                    )),
                }
            }
        }

        Ok(())
    }
}
