mod applications;
mod audio;
mod battery;
mod bluetooth;
mod brightness;
mod control_center;
mod menu;
mod music;
mod network;
mod notifications;
mod runner;
mod search;
mod search_window;
mod session;
mod theme;
mod tray;
mod window_manager;

pub use applications::toggle_config_app;
pub use audio::{
    get_audio_devices, get_audio_volume, set_audio_device, set_audio_volume, toggle_audio_applet,
    toggle_audio_mute,
};
pub use battery::{battery_exists, battery_fetch_info};
pub use bluetooth::toggle_bluetooth_applet;
pub use brightness::{get_brightness_info, set_brightness_info};
pub use control_center::toggle_control_center;
pub use menu::{get_menu_items, toggle_menu};
pub use music::{music_next_track, music_now_playing, music_play_pause, music_previous_track};
pub use network::toggle_network_applet;
pub use notifications::{
    clear_notifications, delete_notification, get_all_notifications, invoke_notification_action,
    send_notify,
};
pub use runner::open_app;
pub use search::{execute_search_result, global_search};
pub use search_window::toggle_search;
pub use session::{detect_display_server, logout, reboot, shutdown, suspend};
pub use theme::toggle_system_theme;
pub use tray::{get_tray_items, init_sni_watcher};
pub use window_manager::{get_windows, toggle_window};
