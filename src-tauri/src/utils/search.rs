use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use once_cell::sync::Lazy;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub description: String,
    pub icon: Option<String>,
    pub category: SearchCategory,
    pub exec: Option<String>,
    pub path: Option<String>,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SearchCategory {
    Application,
    File,
    Action,
}

#[derive(Debug, Clone)]
struct DesktopEntry {
    name: String,
    exec: String,
    icon: Option<String>,
    comment: Option<String>,
    path: PathBuf,
}

// Caché global de aplicaciones
static APP_CACHE: Lazy<Mutex<HashMap<String, DesktopEntry>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static CACHE_TIMESTAMP: Lazy<Mutex<Option<std::time::SystemTime>>> = Lazy::new(|| Mutex::new(None));

/// Parse a .desktop file
fn parse_desktop_file(path: &PathBuf) -> Option<DesktopEntry> {
    let content = fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut comment = None;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();
        
        if line == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        
        if line.starts_with('[') && line != "[Desktop Entry]" {
            in_desktop_entry = false;
        }
        
        if !in_desktop_entry {
            continue;
        }

        if let Some(stripped) = line.strip_prefix("Name=") {
            name = Some(stripped.to_string());
        } else if let Some(stripped) = line.strip_prefix("Exec=") {
            // Remove field codes like %F, %U, etc.
            let cleaned = stripped
                .replace("%f", "")
                .replace("%F", "")
                .replace("%u", "")
                .replace("%U", "")
                .replace("%d", "")
                .replace("%D", "")
                .replace("%n", "")
                .replace("%N", "")
                .replace("%i", "")
                .replace("%c", "")
                .replace("%k", "")
                .replace("%v", "")
                .replace("%m", "")
                .trim()
                .to_string();
            exec = Some(cleaned);
        } else if let Some(stripped) = line.strip_prefix("Icon=") {
            icon = Some(stripped.to_string());
        } else if let Some(stripped) = line.strip_prefix("Comment=") {
            comment = Some(stripped.to_string());
        }
    }

    Some(DesktopEntry {
        name: name?,
        exec: exec?,
        icon,
        comment,
        path: path.clone(),
    })
}

/// Scan and cache desktop applications
fn scan_applications() -> HashMap<String, DesktopEntry> {
    let mut apps = HashMap::new();
    
    let search_paths = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        dirs::home_dir().map(|h| h.join(".local/share/applications")).unwrap_or_default(),
    ];

    for dir in search_paths {
        if !dir.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Some(desktop_entry) = parse_desktop_file(&path) {
                        let id = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        apps.insert(id, desktop_entry);
                    }
                }
            }
        }
    }

    apps
}

/// Refresh application cache if needed
fn ensure_cache_fresh() {
    let mut timestamp = CACHE_TIMESTAMP.lock().unwrap();
    let now = std::time::SystemTime::now();
    
    let should_refresh = match *timestamp {
        None => true,
        Some(last) => {
            // Refresh if cache is older than 5 minutes
            now.duration_since(last)
                .map(|d| d.as_secs() > 300)
                .unwrap_or(true)
        }
    };

    if should_refresh {
        log::info!("[search] Refreshing application cache");
        let apps = scan_applications();
        let mut cache = APP_CACHE.lock().unwrap();
        *cache = apps;
        *timestamp = Some(now);
    }
}

/// Simple fuzzy matching score
fn fuzzy_score(query: &str, text: &str) -> f64 {
    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();
    
    // Exact match
    if text_lower == query_lower {
        return 100.0;
    }
    
    // Starts with query
    if text_lower.starts_with(&query_lower) {
        return 90.0;
    }
    
    // Contains query
    if text_lower.contains(&query_lower) {
        return 70.0;
    }
    
    // Fuzzy matching: all characters present in order
    let mut last_index = 0;
    let mut matches = 0;
    
    for query_char in query_lower.chars() {
        if let Some(pos) = text_lower[last_index..].find(query_char) {
            last_index += pos + 1;
            matches += 1;
        }
    }
    
    if matches == query.len() {
        50.0 / (1.0 + (last_index as f64 - query.len() as f64))
    } else {
        0.0
    }
}

/// Search for applications
pub fn search_applications(query: &str, limit: usize) -> Vec<SearchResult> {
    if query.trim().is_empty() {
        return vec![];
    }

    ensure_cache_fresh();
    
    let cache = APP_CACHE.lock().unwrap();
    let mut results: Vec<SearchResult> = cache
        .iter()
        .filter_map(|(id, entry)| {
            let name_score = fuzzy_score(query, &entry.name);
            let comment_score = entry.comment
                .as_ref()
                .map(|c| fuzzy_score(query, c) * 0.5)
                .unwrap_or(0.0);
            
            let score = name_score.max(comment_score);
            
            if score > 0.0 {
                Some(SearchResult {
                    id: id.clone(),
                    title: entry.name.clone(),
                    description: entry.comment.clone().unwrap_or_default(),
                    icon: entry.icon.clone(),
                    category: SearchCategory::Application,
                    exec: Some(entry.exec.clone()),
                    path: Some(entry.path.to_string_lossy().to_string()),
                    score,
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    results
}

/// Get system actions (power, settings, etc)
pub fn get_system_actions(query: &str) -> Vec<SearchResult> {
    let actions = vec![
        ("shutdown", "Apagar", "Apagar el sistema", "system-shutdown"),
        ("reboot", "Reiniciar", "Reiniciar el sistema", "system-reboot"),
        ("suspend", "Suspender", "Suspender el sistema", "system-suspend"),
        ("lock", "Bloquear", "Bloquear la pantalla", "system-lock-screen"),
        ("logout", "Cerrar sesión", "Cerrar la sesión actual", "system-log-out"),
        ("settings", "Configuración", "Abrir configuración del sistema", "preferences-system"),
    ];

    actions
        .iter()
        .filter_map(|(id, title, desc, icon)| {
            let score = fuzzy_score(query, title).max(fuzzy_score(query, desc));
            if score > 0.0 {
                Some(SearchResult {
                    id: id.to_string(),
                    title: title.to_string(),
                    description: desc.to_string(),
                    icon: Some(icon.to_string()),
                    category: SearchCategory::Action,
                    exec: None,
                    path: None,
                    score,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Main search function combining all sources
pub fn search(query: &str, limit: usize) -> Vec<SearchResult> {
    if query.trim().is_empty() {
        return vec![];
    }

    let mut results = Vec::new();
    
    // Search applications
    let app_results = search_applications(query, limit);
    results.extend(app_results);
    
    // Search system actions
    let action_results = get_system_actions(query);
    results.extend(action_results);
    
    // Sort all results by score
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    
    results
}
