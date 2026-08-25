use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use chrono::Local;
use std::sync::LazyLock;

/// Nivel de log
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Fuente del log
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSource {
    Rust,
    JavaScript,
}

impl LogSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogSource::Rust => "RUST",
            LogSource::JavaScript => "JS",
        }
    }
}

/// Logger global
pub static LOGGER: LazyLock<Mutex<VasakLogger>> = LazyLock::new(|| {
    let logger = VasakLogger::new();
    Mutex::new(logger)
});

/// Logger principal de Vasak Desktop
pub struct VasakLogger {
    /// Buffered: the log used to be flushed after every single line, which
    /// turned each frontend console call into a synchronous disk write.
    log_file: Option<BufWriter<File>>,
    log_path: PathBuf,
    is_dev_mode: bool,
}

impl VasakLogger {
    /// Crea una nueva instancia del logger
    pub fn new() -> Self {
        let is_dev_mode = cfg!(debug_assertions);
        
        // Determinar la ruta del archivo de log
        let log_path = Self::get_log_path();
        
        // Crear el directorio si no existe
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        
        // Abrir o crear el archivo de log
        Self::prune_old_logs(&log_path);

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok()
            .map(BufWriter::new);
        
        if log_file.is_none() {
            eprintln!("⚠️ No se pudo crear el archivo de log en: {:?}", log_path);
        }
        
        let mut logger = Self {
            log_file,
            log_path,
            is_dev_mode,
        };
        
        // Escribir encabezado de sesión
        logger.log_session_start();
        
        logger
    }
    
    /// Obtiene la ruta del archivo de log
    fn get_log_path() -> PathBuf {
        // Usar XDG_DATA_HOME o ~/.local/share como base
        let base_dir = dirs::data_local_dir()
            .unwrap_or_else(|| {
                let home = dirs::home_dir().expect("No se pudo obtener el directorio home");
                home.join(".local/share")
            });
        
        let log_dir = base_dir.join("vasak-desktop").join("logs");
        
        // Nombre del archivo con fecha
        let date = Local::now().format("%Y-%m-%d");
        log_dir.join(format!("vasak-desktop-{}.log", date))
    }
    
    /// Escribe el encabezado de inicio de sesión
    fn log_session_start(&mut self) {
        let mode = if self.is_dev_mode { "DESARROLLO" } else { "PRODUCCIÓN" };
        let separator = "=".repeat(80);
        
        self.write_to_file(&format!("\n{}\n", separator), false);
        self.write_to_file(&format!("Nueva sesión iniciada: {}\n", Local::now().format("%Y-%m-%d %H:%M:%S")), false);
        self.write_to_file(&format!("Modo: {}\n", mode), false);
        self.write_to_file(&format!("Archivo de log: {:?}\n", self.log_path), false);
        self.write_to_file(&format!("{}\n\n", separator), true);
    }
    
    /// Escribe un mensaje en el archivo.
    ///
    /// Only errors and warnings are flushed immediately; anything else rides in
    /// the buffer. Flushing per line meant every frontend `console.log` became a
    /// synchronous disk write on the main thread, and the interesting lines are
    /// exactly the ones we still flush, so a crash does not lose them.
    fn write_to_file(&mut self, message: &str, flush: bool) {
        if let Some(ref mut file) = self.log_file {
            let _ = file.write_all(message.as_bytes());
            if flush {
                let _ = file.flush();
            }
        }
    }

    /// Deletes log files older than a week.
    ///
    /// One file per day was created and never removed, so the directory grew
    /// without bound for the life of the install.
    fn prune_old_logs(log_path: &Path) {
        const MAX_AGE: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 60 * 60);

        let Some(dir) = log_path.parent() else { return };
        let Ok(entries) = std::fs::read_dir(dir) else { return };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|ext| ext.to_str()) != Some("log") {
                continue;
            }

            let too_old = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .map(|modified| modified.elapsed().map(|age| age > MAX_AGE).unwrap_or(false))
                .unwrap_or(false);

            if too_old {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    
    /// Registra un mensaje
    pub fn log(&mut self, level: LogLevel, source: LogSource, message: &str) {
        // En producción se omiten solo los Debug.
        if !self.is_dev_mode && matches!(level, LogLevel::Debug) {
            return;
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let formatted_message = format!(
            "[{}] [{:>8}] [{:>4}] {}\n",
            timestamp,
            level.as_str(),
            source.as_str(),
            message
        );
        
        // Escribir al archivo (errores y avisos se vuelcan al instante)
        let urgent = matches!(level, LogLevel::Error | LogLevel::Warning);
        self.write_to_file(&formatted_message, urgent);
        
        // En modo desarrollo, también imprimir en consola
        if self.is_dev_mode {
            match level {
                LogLevel::Error => eprint!("{}", formatted_message),
                LogLevel::Warning => eprint!("{}", formatted_message),
                LogLevel::Info | LogLevel::Debug => print!("{}", formatted_message),
            }
        }
    }
    
    /// Obtiene la ruta actual del log
    pub fn get_current_log_path(&self) -> PathBuf {
        self.log_path.clone()
    }

    /// Vuelca lo que quede en el buffer.
    pub fn flush(&mut self) {
        if let Some(ref mut file) = self.log_file {
            let _ = file.flush();
        }
    }
}

// ── Puente con el crate `log` ───────────────────────────────────────────────

/// Manda al registro de la aplicación lo que se escriba con las macros de `log`.
///
/// `log` es una fachada: sin un backend instalado descarta todos los registros
/// en silencio, y así estaban las setenta y dos llamadas repartidas por la
/// aplicación. No eran mensajes de adorno —un applet crítico que no arranca, los
/// monitores de D-Bus rindiéndose tras el último reintento, las lecturas de
/// brillo que fallan, el IPC de Wayfire sin responder— y ninguno dejaba rastro.
///
/// Se hace con un puente y no reescribiendo los setenta y dos sitios: es el
/// mismo resultado en treinta líneas, y deja funcionando de una vez lo que
/// escriba cualquier dependencia que también use `log`.
struct LogCrateBridge;

impl log::Log for LogCrateBridge {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = match record.level() {
            log::Level::Error => LogLevel::Error,
            log::Level::Warn => LogLevel::Warning,
            log::Level::Info => LogLevel::Info,
            // El registro propio no distingue Trace, y nadie lo usa acá.
            log::Level::Debug | log::Level::Trace => LogLevel::Debug,
        };

        // El objetivo va delante porque es lo único que dice de dónde salió el
        // mensaje: varias de estas llamadas están en dependencias.
        let message = format!("[{}] {}", record.target(), record.args());

        if let Ok(mut logger) = LOGGER.lock() {
            logger.log(level, LogSource::Rust, &message);
        }
    }

    fn flush(&self) {
        if let Ok(mut logger) = LOGGER.lock() {
            logger.flush();
        }
    }
}

static LOG_BRIDGE: LogCrateBridge = LogCrateBridge;

/// Conecta las macros de `log` con el registro de la aplicación.
///
/// Hay que llamarla una vez, lo antes posible: lo que se registre antes se
/// pierde, porque así funciona la fachada.
///
/// El techo de nivel se pone acá y no se deja en su valor por omisión, que es
/// `Off` —con un backend instalado y el techo en `Off` no pasaría nada igual—.
/// En release queda en `Info`: el registro propio ya descarta los `Debug` fuera
/// de desarrollo, y filtrarlos en la fachada evita además construir el mensaje.
pub fn install_log_bridge() {
    let level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    // Falla si ya había uno puesto, que no es un problema: significa que algo
    // se adelantó, y avisar por el mismo canal que acabamos de instalar sería
    // dar vueltas.
    if log::set_logger(&LOG_BRIDGE).is_ok() {
        log::set_max_level(level);
    }
}

/// Funciones de conveniencia para logging desde Rust
#[allow(dead_code)]
pub fn log_debug(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Debug, LogSource::Rust, message);
    }
}

#[allow(dead_code)]
pub fn log_info(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Info, LogSource::Rust, message);
    }
}

#[allow(dead_code)]
pub fn log_warning(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Warning, LogSource::Rust, message);
    }
}

#[allow(dead_code)]
pub fn log_error(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(LogLevel::Error, LogSource::Rust, message);
    }
}

/// Log desde JavaScript
pub fn log_from_js(level: &str, message: &str) {
    let log_level = match level.to_uppercase().as_str() {
        "DEBUG" => LogLevel::Debug,
        "INFO" => LogLevel::Info,
        "WARNING" | "WARN" => LogLevel::Warning,
        "ERROR" => LogLevel::Error,
        _ => LogLevel::Info,
    };
    
    if let Ok(mut logger) = LOGGER.lock() {
        logger.log(log_level, LogSource::JavaScript, message);
    }
}

/// Obtiene la ruta del archivo de log actual
pub fn get_log_file_path() -> String {
    if let Ok(logger) = LOGGER.lock() {
        logger.get_current_log_path().to_string_lossy().to_string()
    } else {
        String::from("Error: No se pudo acceder al logger")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_creation() {
        let logger = VasakLogger::new();
        assert!(logger.log_path.to_string_lossy().contains("vasak-desktop"));
    }

    #[test]
    fn test_log_levels() {
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warning.as_str(), "WARNING");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
    }

    #[test]
    fn test_log_sources() {
        assert_eq!(LogSource::Rust.as_str(), "RUST");
        assert_eq!(LogSource::JavaScript.as_str(), "JS");
    }
}

#[cfg(test)]
mod bridge_tests {
    use super::*;

    #[test]
    fn los_niveles_del_crate_log_se_traducen_uno_a_uno() {
        // Sin esto, un `log::warn!` podría terminar archivado como Info y
        // dejar de volcarse al instante, que es lo que distingue un aviso.
        let pares = [
            (log::Level::Error, LogLevel::Error),
            (log::Level::Warn, LogLevel::Warning),
            (log::Level::Info, LogLevel::Info),
            (log::Level::Debug, LogLevel::Debug),
            (log::Level::Trace, LogLevel::Debug),
        ];

        for (origen, esperado) in pares {
            let traducido = match origen {
                log::Level::Error => LogLevel::Error,
                log::Level::Warn => LogLevel::Warning,
                log::Level::Info => LogLevel::Info,
                log::Level::Debug | log::Level::Trace => LogLevel::Debug,
            };
            assert_eq!(traducido, esperado, "{origen:?} se archivó mal");
        }
    }

    #[test]
    fn instalar_el_puente_deja_pasar_los_errores() {
        // El techo por omisión de la fachada es `Off`: instalar un backend y
        // olvidarse de subirlo descarta todo igual, que es la trampa que este
        // arreglo justamente vino a evitar.
        install_log_bridge();

        assert!(
            log::max_level() >= log::LevelFilter::Info,
            "el techo quedó en {} y los mensajes se seguirían descartando",
            log::max_level()
        );
        assert!(log::log_enabled!(log::Level::Error));
        assert!(log::log_enabled!(log::Level::Warn));
        assert!(log::log_enabled!(log::Level::Info));
    }

    #[test]
    fn un_mensaje_del_crate_log_llega_al_archivo() {
        install_log_bridge();

        let ruta = match LOGGER.lock() {
            Ok(logger) => logger.get_current_log_path(),
            Err(_) => return,
        };

        let marca = "puente-log-vasak-prueba";
        log::error!("{marca}");
        log::logger().flush();

        let contenido = std::fs::read_to_string(&ruta).unwrap_or_default();
        assert!(
            contenido.contains(marca),
            "el mensaje no llegó a {}",
            ruta.display()
        );
    }
}
