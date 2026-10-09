mod airplane_mode;
mod applets;
mod audio;
mod batch;
mod battery;
mod calendar;
mod compositor;
mod connect;
mod control_center;
mod equalizer;
mod keep_awake;
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
mod wallpaper_assets;
mod wallpaper_colors;
mod wallpaper_picker;
mod weather;
mod window_manager;

pub use airplane_mode::{get_airplane_mode, set_airplane_mode, unblock_radios};
pub use applets::{dismiss_applet, toggle_applet};
pub use audio::{
    get_audio_devices, get_audio_input_devices, get_audio_volume, get_microphone, set_audio_device,
    set_audio_input_device, set_audio_volume, set_microphone_volume, toggle_audio_mute,
    toggle_microphone_mute,
};
pub use batch::batch_invoke;
pub use battery::{battery_exists, battery_fetch_info, get_battery_info};
pub use calendar::{calendar_locations, calendar_occurrences};
pub use compositor::{
    get_keyboard_layout, get_workspaces, next_keyboard_layout, set_panel_input_region,
    switch_workspace,
};
pub use connect::toggle_connect_menu;
pub use control_center::{hide_control_center, toggle_control_center};
pub use equalizer::{
    equalizer_set_enabled, equalizer_set_gain, equalizer_set_preset, equalizer_state,
};
pub use keep_awake::{get_keep_awake, set_keep_awake};
pub use logger::{get_last_log_lines, get_log_file_path, log_from_frontend, read_log_file};
pub use menu::{get_menu_items, set_menu_button, toggle_menu};
pub use music::{
    music_artwork, music_next_track, music_now_playing, music_play_pause, music_players,
    music_previous_track, music_raise, music_select_player, music_set_loop, music_set_position,
    music_set_shuffle, music_set_volume, music_stop,
};
pub use notifications::{
    clear_notifications, delete_notification, get_all_notifications, get_do_not_disturb,
    get_game_mode, invoke_notification_action, send_notify, set_do_not_disturb, set_game_mode,
};
pub use osd::show_osd;
pub use panel::show_panel;
pub use privacy::{privacy_in_use, privacy_stop_screen};
pub use runner::{open_app, open_calendar, open_settings, open_settings_section};
pub use session::{detect_display_server, logout, reboot, shutdown, suspend};
pub use session_popup::toggle_session_popup;
pub use tray::{
    get_launcher_entries, get_tray_items, get_tray_menu, get_tray_popup_data, init_sni_watcher,
    load_dbus_menu_level, open_tray_popup, tray_item_activate, tray_item_secondary_activate,
    tray_menu_item_click, tray_popup_click,
};
pub use twingate::{twingate_authorize, twingate_info};
pub use wallpaper_assets::allow_wallpaper_asset;
pub use wallpaper_colors::wallpaper_pixels;
pub use wallpaper_picker::{
    hide_wallpaper_picker, prepare_wallpaper, toggle_wallpaper_picker, wallpaper_catalog,
};
pub use weather::{
    weather_cached, weather_claim, weather_place, weather_release, weather_store, WeatherCache,
};
pub use window_manager::{get_windows, toggle_window};
