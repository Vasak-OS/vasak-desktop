mod audio;
mod battery;
mod bluetooth;
mod brightness;
mod control_center;
mod menu;
mod music;
mod notifications;
mod runner;
mod session;
mod theme;
mod tray;
mod window_manager;

pub use audio::{get_audio_volume, set_audio_volume, toggle_audio_mute};
pub use battery::{battery_exists, battery_fetch_info};
pub use bluetooth::toggle_bluetooth_applet;
pub use brightness::{get_brightness_info, set_brightness_info};
pub use control_center::toggle_control_center;
pub use menu::{get_menu_items, toggle_menu};
pub use music::{music_play_pause, music_next_track, music_previous_track, music_now_playing};
pub use notifications::{
    clear_notifications, delete_notification, get_all_notifications, send_notify,
};
pub use runner::open_app;
pub use session::{detect_display_server, logout, reboot, shutdown, suspend};
pub use theme::toggle_system_theme;
pub use tray::{get_tray_items, init_sni_watcher};
pub use window_manager::{get_windows, toggle_window};
