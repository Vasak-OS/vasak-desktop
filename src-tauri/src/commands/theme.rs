use std::process::Command;

#[tauri::command]
pub fn toggle_system_theme() -> Result<(), String> {
    let output = Command::new("gsettings")
        .args(&["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        .map_err(|e| format!("Error getting current color scheme: {}", e))?;

    let current_scheme = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Cambiar entre esquemas de color y temas GTK
    if current_scheme == "'prefer-light'" {
        // Cambiar a modo oscuro
        Command::new("gsettings")
            .args(&[
                "set",
                "org.gnome.desktop.interface",
                "color-scheme",
                "prefer-dark",
            ])
            .output()
            .map_err(|e| format!("Error setting color scheme to dark: {}", e))?;

        Command::new("gsettings")
            .args(&[
                "set",
                "org.gnome.desktop.interface",
                "gtk-theme",
                "Adwaita-dark",
            ])
            .output()
            .map_err(|e| format!("Error setting GTK theme to dark: {}", e))?;
    } else {
        // Cambiar a modo claro
        Command::new("gsettings")
            .args(&[
                "set",
                "org.gnome.desktop.interface",
                "color-scheme",
                "prefer-light",
            ])
            .output()
            .map_err(|e| format!("Error setting color scheme to light: {}", e))?;

        Command::new("gsettings")
            .args(&["set", "org.gnome.desktop.interface", "gtk-theme", "Adwaita"])
            .output()
            .map_err(|e| format!("Error setting GTK theme to light: {}", e))?;
    }

    Ok(())
}
