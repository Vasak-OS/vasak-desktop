    use std::sync::{Arc, Mutex};
    use crate::window_manager::WindowManager;
    use std::collections::HashMap;
    use serde::{Deserialize, Serialize};

    pub struct WMState {
        pub(crate) window_manager: Arc<Mutex<WindowManager>>,
    }

    // Estructura para notificaciones reales
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Notification {
        pub id: u32,
        pub app_name: String,
        pub summary: String,
        pub body: String,
        pub app_icon: String,
        pub timestamp: u64,
        pub seen: bool,
        pub urgency: NotificationUrgency,
        pub actions: Vec<String>,
        pub hints: HashMap<String, String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum NotificationUrgency {
        Low,
        Normal,
        Critical,
    }


    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct AppEntry {
        pub category: String,
        pub name: String,
        pub generic: String,
        pub description: String,
        pub icon: String,
        pub keywords: String,
        pub path: String,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct CategoryInfo {
        pub icon: String,
        pub description: String,
        pub apps: Vec<AppEntry>,
    }
