use super::{emit_tray_update, TrayManager};
use crate::structs::{TrayCategory, TrayItem, TrayStatus};
use crate::tray::sni_item::SniItemProxy;
use base64::{engine::general_purpose, Engine as _};
use futures_util::stream::StreamExt;
use tauri::AppHandle;
use zbus::{Connection, MatchRule, MessageStream, MessageType};

const SNI_WATCHER_SERVICE: &str = "org.kde.StatusNotifierWatcher";
const SNI_WATCHER_PATH: &str = "/StatusNotifierWatcher";
const SNI_WATCHER_INTERFACE: &str = "org.kde.StatusNotifierWatcher";

pub struct SniWatcher {
    connection: Connection,
    tray_manager: TrayManager,
    app_handle: AppHandle,
}

impl SniWatcher {
    pub async fn new(
        tray_manager: TrayManager,
        app_handle: AppHandle,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Connection::session().await?;

        // Register as StatusNotifierWatcher
        connection.request_name(SNI_WATCHER_SERVICE).await?;

        Ok(Self {
            connection,
            tray_manager,
            app_handle,
        })
    }

    pub async fn start_watching(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Listen for StatusNotifierItem registrations
        let rule = MatchRule::builder()
            .msg_type(MessageType::MethodCall)
            .interface(SNI_WATCHER_INTERFACE)?
            .member("RegisterStatusNotifierItem")?
            .build();

        let mut stream = MessageStream::for_match_rule(rule, &self.connection, None).await?;

        // Also listen for service name changes
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
                loop {
                    tokio::select! {
                        Some(msg) = stream.next() => {
                            if let Ok(message) = msg {
                                if let Ok(service_name) = message.body().deserialize::<&str>() {
                                    if let Err(e) = Self::register_item(&connection, &tray_manager, &app_handle, service_name).await {
                                        eprintln!("[SNI] Error registrando item {}: {}", service_name, e);
                                    }
                                }
                            }
                        }
                        Some(msg) = name_stream.next() => {
                            if let Ok(message) = msg {
                                if let Ok((name, _old_owner, new_owner)) = message.body().deserialize::<(&str, &str, &str)>() {
                                    if new_owner.is_empty() && name.starts_with("org.kde.StatusNotifierItem") {
                                        Self::unregister_item(&tray_manager, &app_handle, name).await;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        // Discover existing StatusNotifierItems
        self.discover_existing_items().await?;

        Ok(())
    }

    async fn register_item(
        connection: &Connection,
        tray_manager: &TrayManager,
        app_handle: &AppHandle,
        service_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("[SNI] Registrando item: {}", service_name);

        let (bus_name, object_path) = if service_name.contains('/') {
            let parts: Vec<&str> = service_name.splitn(2, '/').collect();
            (parts[0], format!("/{}", parts[1]))
        } else {
            (service_name, "/StatusNotifierItem".to_string())
        };

        let proxy = SniItemProxy::builder(connection)
            .destination(bus_name)?
            .path(object_path)?
            .build()
            .await?;

        let item = Self::create_tray_item_from_proxy(&proxy, service_name).await?;

        {
            let mut manager = tray_manager.write().await;
            manager.insert(service_name.to_string(), item);
        }

        emit_tray_update(app_handle).await;
        Ok(())
    }

    async fn unregister_item(
        tray_manager: &TrayManager,
        app_handle: &AppHandle,
        service_name: &str,
    ) {
        println!("[SNI] Desregistrando item: {}", service_name);

        {
            let mut manager = tray_manager.write().await;
            manager.remove(service_name);
        }

        emit_tray_update(app_handle).await;
    }

    async fn create_tray_item_from_proxy(
        proxy: &SniItemProxy<'_>,
        service_name: &str,
    ) -> Result<TrayItem, Box<dyn std::error::Error>> {
        let id = proxy
            .id()
            .await
            .unwrap_or_else(|_| service_name.to_string());
        let title = proxy.title().await.ok();
        let tooltip = proxy.tool_tip().await.ok();
        let icon_name = proxy.icon_name().await.ok();

        let status = match proxy.status().await.unwrap_or_default().as_str() {
            "Active" => TrayStatus::Active,
            "Passive" => TrayStatus::Passive,
            "NeedsAttention" => TrayStatus::NeedsAttention,
            _ => TrayStatus::Passive,
        };

        let category = match proxy.category().await.unwrap_or_default().as_str() {
            "ApplicationStatus" => TrayCategory::ApplicationStatus,
            "Communications" => TrayCategory::Communications,
            "SystemServices" => TrayCategory::SystemServices,
            "Hardware" => TrayCategory::Hardware,
            _ => TrayCategory::ApplicationStatus,
        };

        let icon_data = Self::get_icon_data(proxy).await;
        let menu_path = proxy.menu().await.ok();

        Ok(TrayItem {
            id,
            service_name: service_name.to_string(),
            icon_name,
            icon_data,
            title,
            tooltip,
            status,
            category,
            menu_path,
        })
    }

    async fn get_icon_data(proxy: &SniItemProxy<'_>) -> Option<String> {
        // Try to get icon pixmap first
        if let Ok(pixmaps) = proxy.icon_pixmap().await {
            if let Some(pixmap) = pixmaps.first() {
                if let Ok(base64_data) = Self::convert_pixmap_to_base64(pixmap) {
                    return Some(base64_data);
                }
            }
        }

        // Fallback to icon theme lookup if icon_name is available
        if let Ok(icon_name) = proxy.icon_name().await {
            return Self::get_icon_from_theme(&icon_name).await;
        }

        None
    }

    fn convert_pixmap_to_base64(
        pixmap: &(i32, i32, Vec<u8>),
    ) -> Result<String, Box<dyn std::error::Error>> {
        let (width, height, data) = pixmap;

        // Convert ARGB to RGBA
        let mut rgba_data = Vec::with_capacity(data.len());
        for chunk in data.chunks(4) {
            if chunk.len() == 4 {
                rgba_data.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
            }
        }

        let img = image::RgbaImage::from_raw(*width as u32, *height as u32, rgba_data)
            .ok_or("Failed to create image")?;

        let mut buffer = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut buffer),
            image::ImageFormat::Png,
        )?;

        Ok(general_purpose::STANDARD.encode(&buffer))
    }

    async fn get_icon_from_theme(icon_name: &str) -> Option<String> {
        // Simple icon theme lookup - you might want to use a proper icon theme library
        let common_paths = [
            format!("/usr/share/icons/hicolor/16x16/apps/{}.png", icon_name),
            format!("/usr/share/icons/hicolor/22x22/apps/{}.png", icon_name),
            format!("/usr/share/icons/hicolor/24x24/apps/{}.png", icon_name),
            format!("/usr/share/pixmaps/{}.png", icon_name),
        ];

        for path in &common_paths {
            if let Ok(data) = tokio::fs::read(path).await {
                return Some(general_purpose::STANDARD.encode(&data));
            }
        }

        None
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
            if name.starts_with("org.kde.StatusNotifierItem") {
                if let Err(e) = Self::register_item(
                    &self.connection,
                    &self.tray_manager,
                    &self.app_handle,
                    &name,
                )
                .await
                {
                    eprintln!("[SNI] Error registrando item existente {}: {}", name, e);
                }
            }
        }

        Ok(())
    }
}
