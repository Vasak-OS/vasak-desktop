#[cfg(test)]
mod tests {
    /// Pruebas para el mapeo de teclas de VasakOS a xbindkeys
    
    fn convert_keys_to_xbindkeys(keys: &str) -> String {
        let mut parts: Vec<String> = Vec::new();
        
        for part in keys.split('+') {
            let trimmed = part.trim();
            let converted = match trimmed {
                "Ctrl" | "Control" => "Control",
                "Alt" => "Alt",
                "Shift" => "Shift",
                "Super" | "Meta" | "Mod4" => "Mod4",
                "Space" => "space",
                "Return" | "Enter" => "Return",
                "Print" => "Print",
                key if key.starts_with('F') && key.len() >= 2 => {
                    if key[1..].parse::<u8>().is_ok() {
                        key
                    } else {
                        &trimmed.to_lowercase()
                    }
                }
                key if key.len() == 1 => &key.to_lowercase(),
                _ => &trimmed.to_lowercase(),
            };
            
            parts.push(converted.to_string());
        }
        
        if parts.len() > 1 {
            let modifiers = &parts[..parts.len() - 1];
            let key = &parts[parts.len() - 1];
            format!("{} + {}", modifiers.join("+"), key)
        } else if parts.len() == 1 {
            parts[0].clone()
        } else {
            keys.to_string()
        }
    }

    #[test]
    fn test_terminal_shortcut() {
        assert_eq!(convert_keys_to_xbindkeys("Ctrl+Alt+T"), "Control+Alt + t");
    }

    #[test]
    fn test_file_manager_shortcut() {
        assert_eq!(convert_keys_to_xbindkeys("Super+E"), "Mod4 + e");
    }

    #[test]
    fn test_lock_shortcut() {
        assert_eq!(convert_keys_to_xbindkeys("Super+L"), "Mod4 + l");
    }

    #[test]
    fn test_screenshot_shortcut() {
        assert_eq!(convert_keys_to_xbindkeys("Print"), "Print");
    }

    #[test]
    fn test_search_shortcut() {
        assert_eq!(convert_keys_to_xbindkeys("Super+Space"), "Mod4 + space");
    }

    #[test]
    fn test_single_super_key() {
        assert_eq!(convert_keys_to_xbindkeys("Super"), "Mod4");
    }

    #[test]
    fn test_config_shortcut() {
        assert_eq!(convert_keys_to_xbindkeys("Super+,"), "Mod4 + ,");
    }

    #[test]
    fn test_function_keys() {
        assert_eq!(convert_keys_to_xbindkeys("F1"), "F1");
        assert_eq!(convert_keys_to_xbindkeys("F12"), "F12");
        assert_eq!(convert_keys_to_xbindkeys("Ctrl+F5"), "Control + F5");
    }

    #[test]
    fn test_shift_combinations() {
        assert_eq!(convert_keys_to_xbindkeys("Shift+F10"), "Shift + F10");
        assert_eq!(convert_keys_to_xbindkeys("Ctrl+Shift+Esc"), "Control+Shift + escape");
    }

    #[test]
    fn test_complex_combination() {
        assert_eq!(convert_keys_to_xbindkeys("Ctrl+Alt+Shift+T"), "Control+Alt+Shift + t");
    }
}
