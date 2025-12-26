pub mod audio_applet;
pub mod bluetooth_applet;
pub mod network_applet;

pub use audio_applet::create_applet_audio_window;
pub use bluetooth_applet::create_applet_bluetooth_window;
pub use network_applet::create_applet_network_window;
