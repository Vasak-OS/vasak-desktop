mod applets;
mod audio;
mod batch;
mod battery;
mod brightness;
mod connect;
mod control_center;
mod logger;
mod menu;
mod music;
mod notifications;
pub mod osd;
mod panel;
mod privacy;
pub mod runner;
mod session;
mod session_popup;
mod tray;
mod twingate;
mod weather;
mod window_manager;

pub use applets::{dismiss_applet, toggle_applet};
pub use audio::{
    get_audio_devices, get_audio_volume, set_audio_device, set_audio_volume, toggle_audio_mute,
};
pub use batch::batch_invoke;
pub use battery::{battery_exists, battery_fetch_info, get_battery_info};
pub use brightness::{get_brightness_info, set_brightness_info};
pub use connect::toggle_connect_menu;
pub use control_center::{hide_control_center, toggle_control_center};
pub use logger::{get_last_log_lines, get_log_file_path, log_from_frontend, read_log_file};
pub use menu::{get_menu_items, set_menu_button, toggle_menu};
pub use music::{
    music_artwork, music_next_track, music_now_playing, music_play_pause, music_players,
    music_previous_track, music_raise, music_select_player, music_set_loop, music_set_position,
    music_set_shuffle, music_set_volume, music_stop,
};
pub use notifications::{
    clear_notifications, delete_notification, get_all_notifications, invoke_notification_action,
    send_notify,
};
pub use osd::show_osd;
pub use panel::show_panel;
pub use privacy::{privacy_in_use, privacy_stop_screen};
pub use runner::{open_app, open_settings, open_settings_section};
pub use session::{detect_display_server, logout, reboot, shutdown, suspend};
pub use session_popup::toggle_session_popup;
pub use tray::{
    get_launcher_entries, get_tray_items, get_tray_menu, get_tray_popup_data, init_sni_watcher,
    load_dbus_menu_level, open_tray_popup,
    tray_item_activate, tray_item_secondary_activate, tray_menu_item_click, tray_popup_click,
};
pub use twingate::{twingate_authorize, twingate_info};
pub use weather::{
    weather_cached, weather_claim, weather_place, weather_release, weather_store, WeatherCache,
};
pub use window_manager::{get_windows, toggle_window};
