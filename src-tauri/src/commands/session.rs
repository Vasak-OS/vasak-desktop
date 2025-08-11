use std::process::Command;
use std::env;

#[tauri::command]
pub fn detect_display_server() -> String {
    if env::var("WAYLAND_DISPLAY").is_ok() {
        "wayland".to_string()
    } else if env::var("DISPLAY").is_ok() {
        "x11".to_string()
    } else {
        "unknown".to_string()
    }
}

#[tauri::command]
pub fn logout(display_server: String) -> Result<(), String> {
    let commands = match display_server.as_str() {
        "wayland" => vec![
            "loginctl terminate-user $USER",
            "pkill -KILL -u $USER",
        ],
        "x11" => vec![
            "loginctl terminate-user $USER",
            "pkill -KILL -u $USER",
            "pkill -f 'gnome-session|kde-session|xfce4-session'",
        ],
        _ => vec!["loginctl terminate-user $USER"],
    };

    for cmd in commands {
        let _ = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output();
    }
    Ok(())
}

#[tauri::command]
pub fn shutdown() -> Result<(), String> {
    let commands = vec![
        "systemctl poweroff",
        "shutdown -h now",
        "poweroff",
    ];

    for cmd in commands {
        if Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .status()
            .is_ok() {
            return Ok(());
        }
    }
    Err("No se pudo ejecutar shutdown".to_string())
}

#[tauri::command]
pub fn reboot() -> Result<(), String> {
    let commands = vec![
        "systemctl reboot",
        "shutdown -r now",
        "reboot",
    ];

    for cmd in commands {
        if Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .status()
            .is_ok() {
            return Ok(());
        }
    }
    Err("No se pudo ejecutar reboot".to_string())
}

#[tauri::command]
pub fn suspend(display_server: String) -> Result<(), String> {
    let commands = match display_server.as_str() {
        "wayland" => vec![
            "systemctl suspend",
            "loginctl suspend",
        ],
        "x11" => vec![
            "systemctl suspend",
            "loginctl suspend",
            "dbus-send --system --print-reply --dest=org.freedesktop.UPower /org/freedesktop/UPower org.freedesktop.UPower.Suspend",
        ],
        _ => vec!["systemctl suspend"],
    };

    for cmd in commands {
        if Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .status()
            .is_ok() {
            return Ok(());
        }
    }
    Err("No se pudo ejecutar suspend".to_string())
}