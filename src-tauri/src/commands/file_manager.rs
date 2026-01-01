use std::fs;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct FileEntry {
    name: String,
    is_dir: bool,
    size: String,
    path: String,
}

#[tauri::command]
pub fn read_directory(path: String) -> Result<Vec<FileEntry>, String> {
    // Handle home directory expansion if needed, but for now strict paths.
    // Ideally the frontend sends absolute paths.
    
    let dir_path = Path::new(&path);
    if !dir_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let mut entries = Vec::new();

    match fs::read_dir(dir_path) {
        Ok(read_dir) => {
            for entry_result in read_dir {
                if let Ok(entry) = entry_result {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let file_path = entry.path().to_string_lossy().to_string();
                    
                    let metadata = entry.metadata().map_err(|e| e.to_string())?;
                    let is_dir = metadata.is_dir();
                    
                    let size = if is_dir {
                        "-".to_string()
                    } else {
                        format_size(metadata.len())
                    };

                    entries.push(FileEntry {
                        name: file_name,
                        is_dir,
                        size,
                        path: file_path,
                    });
                }
            }
        }
        Err(e) => return Err(e.to_string()),
    }

    // Sort: Directories first, then files. Alphabetical within groups.
    entries.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        } else {
            b.is_dir.cmp(&a.is_dir) // true (dir) > false (file)
        }
    });

    Ok(entries)
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
