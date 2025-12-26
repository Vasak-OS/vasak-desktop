use crate::search;

#[tauri::command]
pub async fn global_search(query: String, limit: Option<usize>) -> Vec<search::SearchResult> {
    let max_results = limit.unwrap_or(50).min(100); // Max 100 results
    search::search(&query, max_results)
}

#[tauri::command]
pub async fn execute_search_result(id: String, category: String, exec: Option<String>) -> Result<String, String> {
    match category.as_str() {
        "application" => {
            if let Some(cmd) = exec {
                // Parse command and arguments - clone to extend lifetime
                let cmd_owned = cmd.clone();
                let parts: Vec<String> = cmd_owned.split_whitespace().map(|s| s.to_string()).collect();
                if parts.is_empty() {
                    return Err("Empty command".to_string());
                }

                // Launch application in background
                tokio::spawn(async move {
                    let _ = tokio::process::Command::new(&parts[0])
                        .args(&parts[1..])
                        .spawn();
                });

                Ok(format!("Launched: {}", cmd))
            } else {
                Err("No exec command".to_string())
            }
        }
        "action" => {
            // Handle system actions
            match id.as_str() {
                "shutdown" => {
                    tokio::spawn(async {
                        let _ = tokio::process::Command::new("systemctl")
                            .args(&["poweroff"])
                            .spawn();
                    });
                    Ok("Shutting down...".to_string())
                }
                "reboot" => {
                    tokio::spawn(async {
                        let _ = tokio::process::Command::new("systemctl")
                            .args(&["reboot"])
                            .spawn();
                    });
                    Ok("Rebooting...".to_string())
                }
                "suspend" => {
                    tokio::spawn(async {
                        let _ = tokio::process::Command::new("systemctl")
                            .args(&["suspend"])
                            .spawn();
                    });
                    Ok("Suspending...".to_string())
                }
                "lock" => {
                    tokio::spawn(async {
                        let _ = tokio::process::Command::new("loginctl")
                            .args(&["lock-session"])
                            .spawn();
                    });
                    Ok("Locking screen...".to_string())
                }
                "logout" => {
                    tokio::spawn(async {
                        let _ = tokio::process::Command::new("loginctl")
                            .args(&["terminate-user", &std::env::var("USER").unwrap_or_default()])
                            .spawn();
                    });
                    Ok("Logging out...".to_string())
                }
                "settings" => {
                    // TODO: Open VasakOS settings app
                    Ok("Opening settings...".to_string())
                }
                _ => Err(format!("Unknown action: {}", id))
            }
        }
        _ => Err(format!("Unknown category: {}", category))
    }
}
