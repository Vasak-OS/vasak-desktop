use zbus::proxy;

#[proxy(
    interface = "org.kde.StatusNotifierItem",
    default_service = "org.kde.StatusNotifierItem",
    default_path = "/StatusNotifierItem"
)]
trait SniItem {
    /// Activate method
    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// SecondaryActivate method  
    fn secondary_activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// Scroll method
    fn scroll(&self, delta: i32, orientation: &str) -> zbus::Result<()>;

    /// Category property
    #[zbus(property)]
    fn category(&self) -> zbus::Result<String>;

    /// Id property
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    /// Title property
    #[zbus(property)]
    fn title(&self) -> zbus::Result<String>;

    /// Status property
    #[zbus(property)]
    fn status(&self) -> zbus::Result<String>;

    /// WindowId property
    #[zbus(property)]
    fn window_id(&self) -> zbus::Result<u32>;

    /// IconName property
    #[zbus(property)]
    fn icon_name(&self) -> zbus::Result<String>;

    /// IconPixmap property
    #[zbus(property)]
    fn icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;

    /// OverlayIconName property
    #[zbus(property)]
    fn overlay_icon_name(&self) -> zbus::Result<String>;

    /// OverlayIconPixmap property
    #[zbus(property)]
    fn overlay_icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;

    /// AttentionIconName property
    #[zbus(property)]
    fn attention_icon_name(&self) -> zbus::Result<String>;

    /// AttentionIconPixmap property
    #[zbus(property)]
    fn attention_icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;

    /// AttentionMovieName property
    #[zbus(property)]
    fn attention_movie_name(&self) -> zbus::Result<String>;

    /// ToolTip property
    #[zbus(property)]
    fn tool_tip(&self) -> zbus::Result<String>;

    /// Menu property
    #[zbus(property)]
    fn menu(&self) -> zbus::Result<String>;

    /// ItemIsMenu property
    #[zbus(property)]
    fn item_is_menu(&self) -> zbus::Result<bool>;
}
