use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpu: Option<GpuInfo>,
    pub system: SystemDetails,
    pub temperature: Option<TemperatureInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CpuInfo {
    pub model: String,
    pub cores: u32,
    pub usage: f32,
    pub frequency: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryInfo {
    pub total_gb: f64,
    pub used_gb: f64,
    pub available_gb: f64,
    pub usage_percent: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GpuInfo {
    pub model: String,
    pub vendor: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemDetails {
    pub hostname: String,
    pub kernel: String,
    pub os_name: String,
    pub os_version: String,
    pub display_server: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemperatureInfo {
    pub cpu_temp: Option<f32>,
    pub sensors: Vec<SensorReading>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SensorReading {
    pub name: String,
    pub temp: f32,
    pub label: String,
}

/// Obtiene el modelo de CPU desde /proc/cpuinfo
fn get_cpu_model() -> String {
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("model name"))
                .and_then(|line| line.split(':').nth(1))
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "Unknown CPU".to_string())
}

/// Cuenta los núcleos de CPU
fn get_cpu_cores() -> u32 {
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .map(|content| content.lines().filter(|line| line.starts_with("processor")).count() as u32)
        .unwrap_or(1)
}

/// Obtiene el uso actual de CPU (promedio)
fn get_cpu_usage() -> f32 {
    // Leer /proc/stat para calcular uso de CPU
    if let Ok(content) = fs::read_to_string("/proc/stat") {
        if let Some(line) = content.lines().next() {
            if line.starts_with("cpu ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let user: u64 = parts[1].parse().unwrap_or(0);
                    let nice: u64 = parts[2].parse().unwrap_or(0);
                    let system: u64 = parts[3].parse().unwrap_or(0);
                    let idle: u64 = parts[4].parse().unwrap_or(0);

                    let total = user + nice + system + idle;
                    let active = user + nice + system;

                    if total > 0 {
                        return (active as f32 / total as f32) * 100.0;
                    }
                }
            }
        }
    }
    0.0
}

/// Obtiene la frecuencia actual de CPU en GHz
fn get_cpu_frequency() -> Option<f32> {
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("cpu MHz"))
                .and_then(|line| line.split(':').nth(1))
                .and_then(|s| s.trim().parse::<f32>().ok())
                .map(|mhz| mhz / 1000.0) // Convertir a GHz
        })
}

/// Obtiene información de memoria desde /proc/meminfo
fn get_memory_info() -> MemoryInfo {
    let content = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    
    let mut total_kb = 0u64;
    let mut available_kb = 0u64;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            total_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        } else if line.starts_with("MemAvailable:") {
            available_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        }
    }

    let total_gb = total_kb as f64 / 1024.0 / 1024.0;
    let available_gb = available_kb as f64 / 1024.0 / 1024.0;
    let used_gb = total_gb - available_gb;
    let usage_percent = if total_gb > 0.0 {
        (used_gb / total_gb * 100.0) as f32
    } else {
        0.0
    };

    MemoryInfo {
        total_gb,
        used_gb,
        available_gb,
        usage_percent,
    }
}

/// Obtiene información de GPU usando lspci
fn get_gpu_info() -> Option<GpuInfo> {
    Command::new("lspci")
        .output()
        .ok()
        .and_then(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .lines()
                .find(|line| line.to_lowercase().contains("vga compatible") || line.to_lowercase().contains("3d controller"))
                .map(|line| {
                    let parts: Vec<&str> = line.split(':').collect();
                    let info = if parts.len() >= 3 {
                        parts[2].trim()
                    } else {
                        line
                    };

                    let vendor = if info.to_lowercase().contains("nvidia") {
                        "NVIDIA"
                    } else if info.to_lowercase().contains("amd") || info.to_lowercase().contains("ati") {
                        "AMD"
                    } else if info.to_lowercase().contains("intel") {
                        "Intel"
                    } else {
                        "Unknown"
                    };

                    GpuInfo {
                        model: info.to_string(),
                        vendor: vendor.to_string(),
                    }
                })
        })
}

/// Obtiene detalles del sistema
fn get_system_details() -> SystemDetails {
    let hostname = fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "localhost".to_string());

    let kernel = Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let os_name = fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("PRETTY_NAME="))
                .map(|line| line.trim_start_matches("PRETTY_NAME=\"").trim_end_matches('"').to_string())
        })
        .unwrap_or_else(|| "Linux".to_string());

    let os_version = fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("VERSION="))
                .map(|line| line.trim_start_matches("VERSION=\"").trim_end_matches('"').to_string())
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let display_server = if std::env::var("WAYLAND_DISPLAY").is_ok() {
        "Wayland".to_string()
    } else if std::env::var("DISPLAY").is_ok() {
        "X11".to_string()
    } else {
        "Unknown".to_string()
    };

    let uptime_seconds = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|content| {
            content
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<f64>().ok())
                .map(|s| s as u64)
        })
        .unwrap_or(0);

    SystemDetails {
        hostname,
        kernel,
        os_name,
        os_version,
        display_server,
        uptime_seconds,
    }
}

/// Obtiene temperaturas del sistema (si están disponibles)
fn get_temperature_info() -> Option<TemperatureInfo> {
    let mut sensors = Vec::new();
    let mut cpu_temp = None;

    // Intentar leer desde /sys/class/thermal
    if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with("thermal_zone")).unwrap_or(false) {
                if let Ok(temp_str) = fs::read_to_string(path.join("temp")) {
                    if let Ok(temp_millis) = temp_str.trim().parse::<i32>() {
                        let temp = temp_millis as f32 / 1000.0;
                        
                        let type_name = fs::read_to_string(path.join("type"))
                            .ok()
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|| "Unknown".to_string());

                        if type_name.to_lowercase().contains("cpu") || type_name.to_lowercase().contains("x86") {
                            cpu_temp = Some(temp);
                        }

                        sensors.push(SensorReading {
                            name: path.file_name().unwrap().to_string_lossy().to_string(),
                            temp,
                            label: type_name,
                        });
                    }
                }
            }
        }
    }

    if sensors.is_empty() {
        None
    } else {
        Some(TemperatureInfo { cpu_temp, sensors })
    }
}

#[tauri::command]
pub fn get_system_info() -> Result<SystemInfo, String> {
    Ok(SystemInfo {
        cpu: CpuInfo {
            model: get_cpu_model(),
            cores: get_cpu_cores(),
            usage: get_cpu_usage(),
            frequency: get_cpu_frequency(),
        },
        memory: get_memory_info(),
        gpu: get_gpu_info(),
        system: get_system_details(),
        temperature: get_temperature_info(),
    })
}

#[tauri::command]
pub fn get_cpu_usage_only() -> Result<f32, String> {
    Ok(get_cpu_usage())
}

#[tauri::command]
pub fn get_memory_usage_only() -> Result<MemoryInfo, String> {
    Ok(get_memory_info())
}
