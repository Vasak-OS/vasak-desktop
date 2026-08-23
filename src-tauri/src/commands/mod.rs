mod audio;
mod batch;
mod battery;
mod bluetooth;
mod brightness;
mod connect;
mod control_center;
mod logger;
mod menu;
pub mod osd;
mod panel;
mod music;
mod network;
mod notifications;
pub mod runner;
mod search;
mod search_window;
mod session;
mod session_popup;
mod tray;
mod twingate;
mod window_manager;

pub use batch::batch_invoke;
pub use audio::{
    get_audio_devices, get_audio_volume, set_audio_device, set_audio_volume, toggle_audio_applet,
    toggle_audio_mute,
};
pub use battery::{battery_exists, battery_fetch_info, get_battery_info};
pub use bluetooth::toggle_bluetooth_applet;
pub use brightness::{get_brightness_info, set_brightness_info};
pub use connect::toggle_connect_menu;
pub use control_center::{hide_control_center, toggle_control_center};
pub use logger::{log_from_frontend, get_log_file_path, read_log_file, get_last_log_lines};
pub use menu::{get_menu_items, toggle_menu};
pub use osd::show_osd;
pub use panel::show_panel;
pub use music::{music_next_track, music_now_playing, music_play_pause, music_previous_track};
pub use network::toggle_network_applet;
pub use notifications::{
    clear_notifications, delete_notification, get_all_notifications, invoke_notification_action,
    send_notify,
};
pub use runner::{open_app, open_settings};
pub use search::{execute_search_result, global_search};
pub use search_window::toggle_search;
pub use session::{detect_display_server, logout, reboot, shutdown, suspend};
pub use session_popup::toggle_session_popup;
pub use tray::{
    get_tray_items, get_tray_menu, get_tray_popup_data, init_sni_watcher, open_tray_popup,
    tray_item_activate, tray_item_secondary_activate, tray_menu_item_click, tray_popup_click,
};
pub use twingate::{twingate_authorize, twingate_info};
pub use window_manager::{get_windows, toggle_window};

