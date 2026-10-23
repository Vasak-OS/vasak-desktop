//! Los contadores del sistema para el widget de estadísticas (vasak-desktop#269).
//!
//! Salen de `/proc` y `/sys`, archivos de texto del kernel: no hay servicio al
//! que preguntarle. El comando devuelve los contadores **crudos**; la tasa de
//! CPU es la diferencia entre dos lecturas, que hace la interfaz (el backend no
//! guarda estado entre pedidos y el widget sólo pregunta mientras se ve).

use serde::Serialize;
use std::fs;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Default)]
pub struct SystemStats {
    /// Tiempo de CPU total y ocioso, en ticks (`/proc/stat`, línea `cpu`).
    pub cpu_total: u64,
    pub cpu_idle: u64,
    /// Memoria total y disponible, en KiB (`/proc/meminfo`).
    pub mem_total_kib: u64,
    pub mem_available_kib: u64,
    /// La temperatura más alta de la máquina, en °C; `None` sin sensores.
    pub temperature_c: Option<f32>,
}

/// `(total, ocioso)` de la línea agregada `cpu` de `/proc/stat`.
///
/// El ocioso suma `idle` e `iowait`: esperar disco no es trabajar.
pub fn parse_cpu(stat: &str) -> Option<(u64, u64)> {
    let line = stat.lines().find(|l| l.starts_with("cpu "))?;
    let fields: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .map(|f| f.parse().ok())
        .collect::<Option<_>>()?;
    if fields.len() < 5 {
        return None;
    }
    // user nice system idle iowait irq softirq steal …; guest ya está en user.
    let total: u64 = fields.iter().take(8).sum();
    Some((total, fields[3] + fields[4]))
}

/// `(total, disponible)` en KiB de `/proc/meminfo`.
pub fn parse_meminfo(meminfo: &str) -> Option<(u64, u64)> {
    let value = |key: &str| -> Option<u64> {
        meminfo
            .lines()
            .find_map(|l| l.strip_prefix(key))?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    };
    Some((value("MemTotal:")?, value("MemAvailable:")?))
}

/// La milésima de grado de `thermal_zone*/temp` y `hwmon*/temp*_input` a °C.
/// Un valor fuera de lo creíble (sensores sin conectar dan 0 o -273) se descarta.
pub fn celsius(millidegrees: &str) -> Option<f32> {
    let degrees = millidegrees.trim().parse::<i64>().ok()? as f32 / 1000.0;
    (1.0..=150.0).contains(&degrees).then_some(degrees)
}

fn hottest() -> Option<f32> {
    let mut files = Vec::new();
    for (dir, prefix, suffix) in [
        ("/sys/class/thermal", "thermal_zone", "temp"),
        ("/sys/class/hwmon", "hwmon", ""),
    ] {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if !entry.file_name().to_string_lossy().starts_with(prefix) {
                continue;
            }
            if suffix.is_empty() {
                if let Ok(inner) = fs::read_dir(entry.path()) {
                    files.extend(inner.flatten().map(|f| f.path()).filter(|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.starts_with("temp") && n.ends_with("_input"))
                    }));
                }
            } else {
                files.push(entry.path().join(suffix));
            }
        }
    }
    files
        .iter()
        .filter_map(|p| fs::read_to_string(p).ok())
        .filter_map(|s| celsius(&s))
        .fold(None, |max: Option<f32>, t| {
            Some(max.map_or(t, |m| m.max(t)))
        })
}

/// Una lectura cruda de `/proc` y `/sys`.
///
/// La comparten el comando —que pregunta el widget de escritorio mientras se
/// ve— y el applet de recursos del panel (vasak-desktop#216), que la empuja por
/// un evento. Una sola lectura para los dos: dos lecturas distintas de `/proc`
/// se separan con el tiempo y mostrarían números que no coinciden.
pub fn read() -> Result<SystemStats, String> {
    let (cpu_total, cpu_idle) = fs::read_to_string("/proc/stat")
        .map_err(|e| e.to_string())
        .and_then(|s| parse_cpu(&s).ok_or_else(|| "/proc/stat ilegible".to_string()))?;
    let (mem_total_kib, mem_available_kib) = fs::read_to_string("/proc/meminfo")
        .map_err(|e| e.to_string())
        .and_then(|s| parse_meminfo(&s).ok_or_else(|| "/proc/meminfo ilegible".to_string()))?;
    Ok(SystemStats {
        cpu_total,
        cpu_idle,
        mem_total_kib,
        mem_available_kib,
        temperature_c: hottest(),
    })
}

#[tauri::command]
pub fn system_stats() -> Result<SystemStats, String> {
    read()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_cpu_suma_los_campos_y_cuenta_iowait_como_ocio() {
        let stat = "cpu  100 0 50 800 50 0 0 0 0 0\ncpu0 1 1 1 1 1\n";
        assert_eq!(parse_cpu(stat), Some((1000, 850)));
    }

    #[test]
    fn una_linea_de_cpu_rota_no_inventa_numeros() {
        assert_eq!(parse_cpu("cpu  1 2 3\n"), None);
        assert_eq!(parse_cpu("intr 1 2 3\n"), None);
        assert_eq!(parse_cpu("cpu  a b c d e\n"), None);
    }

    #[test]
    fn la_memoria_toma_el_total_y_el_disponible() {
        let meminfo = "MemTotal:       16000000 kB\nMemFree:  1 kB\nMemAvailable:    8000000 kB\n";
        assert_eq!(parse_meminfo(meminfo), Some((16_000_000, 8_000_000)));
        assert_eq!(parse_meminfo("MemTotal: 1 kB\n"), None);
    }

    #[test]
    fn la_temperatura_descarta_lo_increible() {
        assert_eq!(celsius("45000\n"), Some(45.0));
        assert_eq!(celsius("0"), None);
        assert_eq!(celsius("-273150"), None);
        assert_eq!(celsius("basura"), None);
    }

    #[test]
    fn leer_el_sistema_real_da_memoria() {
        // Linux siempre tiene /proc/meminfo; la temperatura puede faltar (VM, CI).
        let stats = system_stats().expect("lee /proc");
        assert!(stats.mem_total_kib > 0);
        assert!(stats.mem_available_kib <= stats.mem_total_kib);
    }

    #[test]
    fn la_lectura_del_applet_lleva_las_claves_que_lee_la_interfaz() {
        // El applet de recursos emite `read()` tal cual por `system-resources` y
        // `system-stats.ts` lee estas cinco claves: renombrar una la dejaría en
        // cero sin que nada avise.
        let json = serde_json::to_value(read().expect("lee /proc")).unwrap();
        for clave in [
            "cpu_total",
            "cpu_idle",
            "mem_total_kib",
            "mem_available_kib",
            "temperature_c",
        ] {
            assert!(json.get(clave).is_some(), "falta la clave {clave}");
        }
    }
}
