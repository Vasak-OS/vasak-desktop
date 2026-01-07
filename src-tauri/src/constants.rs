// Paths del sistema
pub const BACKLIGHT_PATH: &str = "/sys/class/backlight";

// Timeouts
pub const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 3;

// Polling intervals para audio
pub const AUDIO_SLOW_POLL_MS: u64 = 2000;
pub const AUDIO_FAST_POLL_MS: u64 = 500;
pub const AUDIO_FAST_POLL_ITERATIONS: u32 = 10;

// D-Bus
pub const DBUS_SERVICE_NAME: &str = "org.vasak.os.Desktop";

// Comandos del sistema
pub const CMD_WPCTL: &str = "wpctl";
pub const CMD_BRIGHTNESSCTL: &str = "brightnessctl";
pub const CMD_XRANDR: &str = "xrandr";

// Atoms de X11
pub const X11_ATOM_NET_WM_STATE_STICKY: &str = "_NET_WM_STATE_STICKY";
pub const X11_ATOM_NET_WM_STATE_SKIP_TASKBAR: &str = "_NET_WM_STATE_SKIP_TASKBAR";
pub const X11_ATOM_NET_WM_STATE_SKIP_PAGER: &str = "_NET_WM_STATE_SKIP_PAGER";
pub const X11_ATOM_NET_WM_STATE_ABOVE: &str = "_NET_WM_STATE_ABOVE";
pub const X11_ATOM_TYPE: &str = "ATOM";
