mod audio;
mod brightness;
mod control_center;
mod menu;
mod notifications;
mod runner;
mod session;
mod window_manager;

pub use audio::{get_audio_volume, set_audio_volume, toggle_audio_mute};
pub use brightness::{get_brightness_info, set_brightness_info};
pub use control_center::toggle_control_center;
pub use menu::{get_menu_items, toggle_menu};
pub use notifications::{
    clear_notifications, delete_notification, get_all_notifications, send_notify,
};
pub use runner::open_app;
pub use session::{detect_display_server, logout, reboot, shutdown, suspend};
pub use window_manager::{get_windows, toggle_window};
