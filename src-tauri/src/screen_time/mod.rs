//! El tiempo de pantalla por aplicación (vasak-desktop#150).
//!
//! Cuánto tiempo estuvo enfocada cada aplicación, por día, para el tablero que
//! abre el centro de control. Lo mide el escritorio y no un demonio aparte: ya
//! le habla al compositor por el socket de Wayfire para la lista de ventanas, y
//! un proceso más para preguntar lo mismo sería otra conexión, otro servicio y
//! otro permiso para lo mismo.
//!
//! - [`tracker`]: la contabilidad, pura (qué cuenta, el descuento de la
//!   inactividad, la medianoche).
//! - [`store`]: un archivo JSON por día en
//!   `$XDG_DATA_HOME/vasak-desktop/screen-time/`, con escritura atómica.
//! - [`activity`]: el bloqueo (por `/proc`) y la inactividad
//!   (`ext-idle-notify-v1`, tres minutos).
//!
//! # Cómo corre
//!
//! Un hilo da una vuelta cada [`TICK`]: pregunta al compositor qué ventana
//! está enfocada y si hay un programa de bloqueo, y se lo pasa al tracker. Los
//! avisos de inactividad llegan por un canal desde el hilo de Wayland y se
//! atienden en el momento. Cada [`SAVE_EVERY`] se guarda lo que ya no puede
//! descontarse; al cerrar la sesión desde el escritorio (`logout`, `shutdown`,
//! `reboot`) se guarda todo ([`flush`]).
//!
//! # Privacidad
//!
//! Nada sale de la máquina: los archivos son del usuario y nadie más los lee.
//! Se apaga con `screen_time.enabled: false` en `vasak.conf` —lo escribe el
//! interruptor del tablero, y lo va a escribir Configuración— y se borra con
//! «Borrar historial» ([`clear`], también por D-Bus: `ClearScreenTime`).
//! Apagado no cuenta nada: lo de antes de apagarlo se guarda y se deja de
//! mirar el foco.

pub mod activity;
pub mod store;
pub mod tracker;

use chrono::{Duration as ChronoDuration, Local, NaiveDate, Utc};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Listener};

use crate::logger::{log_error, log_info};
use crate::window_manager::app_icon;
use activity::IdleEvent;
use store::Store;
use tracker::{merge, Tracker, Usage};

/// Cada cuánto se mira el foco. El error de un cambio de aplicación es, como
/// mucho, esto.
pub const TICK: Duration = Duration::from_secs(2);

/// Más que esto entre dos vueltas es un salto del reloj (una suspensión sin
/// bloqueo, el proceso detenido): lo del medio no cuenta.
const MAX_GAP: Duration = Duration::from_secs(30);

/// Cada cuánto se guarda lo que ya no puede descontarse.
const SAVE_EVERY: Duration = Duration::from_secs(60);

struct Recorder {
    tracker: Mutex<Tracker<Local>>,
    store: Option<Store>,
    /// Lo entregado por el tracker que todavía no llegó al disco (si el disco
    /// falló, se reintenta en la próxima vuelta).
    unsaved: Mutex<Usage>,
    enabled: AtomicBool,
}

static RECORDER: OnceLock<Arc<Recorder>> = OnceLock::new();

/// Si `vasak.conf` deja contar: sí, salvo un `screen_time.enabled: false`
/// explícito. Un archivo ilegible o sin la clave no apaga nada.
pub fn enabled_from_json(content: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .as_ref()
        .and_then(|config| config.get("screen_time"))
        .and_then(|section| section.get("enabled"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
}

fn read_enabled() -> bool {
    crate::panel_position::config_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|content| enabled_from_json(&content))
        .unwrap_or(true)
}

/// Arranca el registro. Se llama una vez, desde el `setup`.
pub fn start(app: &AppHandle) {
    let store = Store::default_dir().map(Store::new);
    if store.is_none() {
        log_error("[tiempo de pantalla] sin $XDG_DATA_HOME ni carpeta personal: no se guarda nada");
    }
    let recorder = Arc::new(Recorder {
        tracker: Mutex::new(Tracker::new(
            Local,
            ChronoDuration::from_std(activity::IDLE_THRESHOLD)
                .unwrap_or(ChronoDuration::minutes(3)),
            ChronoDuration::from_std(MAX_GAP).unwrap_or(ChronoDuration::seconds(30)),
        )),
        store,
        unsaved: Mutex::new(Usage::new()),
        enabled: AtomicBool::new(read_enabled()),
    });
    if RECORDER.set(recorder.clone()).is_err() {
        return;
    }

    // Prender o apagar desde la configuración, en el acto.
    app.listen("config-changed", |_| {
        if let Some(recorder) = RECORDER.get() {
            let enabled = read_enabled();
            let was = recorder.enabled.swap(enabled, Ordering::SeqCst);
            if was && !enabled {
                log_info("[tiempo de pantalla] apagado desde la configuración");
                recorder.save_everything();
            } else if !was && enabled {
                log_info("[tiempo de pantalla] prendido desde la configuración");
            }
        }
    });

    let (tx, rx) = channel();
    activity::watch_idle(tx);
    let spawned = std::thread::Builder::new()
        .name("screen-time".into())
        .spawn(move || run(recorder, rx));
    if let Err(error) = spawned {
        log_error(&format!(
            "[tiempo de pantalla] no se pudo arrancar: {error}"
        ));
    }
}

fn run(recorder: Arc<Recorder>, idle: Receiver<IdleEvent>) {
    let mut last_save = Instant::now();
    let mut next_tick = Instant::now();
    loop {
        let wait = next_tick.saturating_duration_since(Instant::now());
        match idle.recv_timeout(wait) {
            Ok(event) => {
                recorder.on_idle(event);
                continue;
            }
            Err(RecvTimeoutError::Timeout) => {}
            // Sin hilo de inactividad (el compositor no ofrece el protocolo):
            // el registro sigue, a vueltas fijas.
            Err(RecvTimeoutError::Disconnected) => std::thread::sleep(wait),
        }
        next_tick = Instant::now() + TICK;

        if recorder.enabled.load(Ordering::SeqCst) {
            let focused = focused_app();
            let locked = activity::screen_locked();
            recorder
                .tracker
                .lock()
                .tick(Utc::now(), focused.as_deref(), locked);
        }

        if last_save.elapsed() >= SAVE_EVERY {
            last_save = Instant::now();
            recorder.save_finalized();
        }
    }
}

/// La aplicación enfocada según el compositor, o `None` si no se pudo saber.
fn focused_app() -> Option<String> {
    tauri::async_runtime::block_on(async {
        let client = crate::window_manager::wayfire_ipc::get_wayfire_client().await?;
        let views = client.list_views_typed().await.ok()?;
        crate::window_manager::wayland::focused_app_id(&views)
    })
}

impl Recorder {
    fn on_idle(&self, event: IdleEvent) {
        let now = Utc::now();
        let mut tracker = self.tracker.lock();
        match event {
            IdleEvent::Idle => {
                let since = now
                    - ChronoDuration::from_std(activity::IDLE_THRESHOLD)
                        .unwrap_or(ChronoDuration::minutes(3));
                tracker.idle_started(now, since);
            }
            IdleEvent::Active => tracker.idle_ended(),
        }
    }

    fn save_finalized(&self) {
        let done = self.tracker.lock().finalize(Utc::now());
        self.save(done);
    }

    fn save_everything(&self) {
        let done = self.tracker.lock().finalize_all(Utc::now());
        self.save(done);
    }

    fn save(&self, done: Usage) {
        let mut unsaved = self.unsaved.lock();
        merge(&mut unsaved, &done);
        if unsaved.is_empty() {
            return;
        }
        let Some(store) = &self.store else {
            return;
        };
        match store.add(&unsaved) {
            Ok(()) => unsaved.clear(),
            Err(error) => log_error(&format!(
                "[tiempo de pantalla] no se pudo guardar (se reintenta): {error}"
            )),
        }
    }

    /// Lo guardado más lo que todavía no llegó al disco, de `from` a `to`.
    fn usage(&self, from: NaiveDate, to: NaiveDate) -> Usage {
        let mut usage = self
            .store
            .as_ref()
            .map(|store| store.range(from, to))
            .unwrap_or_default();
        merge(&mut usage, &self.unsaved.lock());
        merge(&mut usage, &self.tracker.lock().live(Utc::now()));
        usage.retain(|day, apps| *day >= from && *day <= to && !apps.is_empty());
        usage
    }

    fn clear(&self) -> Result<(), String> {
        self.tracker.lock().discard();
        self.unsaved.lock().clear();
        match &self.store {
            Some(store) => store.clear().map_err(|error| error.to_string()),
            None => Ok(()),
        }
    }
}

/// Guarda todo lo contado, sin esperar a la ventana de descuento: antes de
/// cerrar la sesión o apagar.
pub fn flush() {
    if let Some(recorder) = RECORDER.get() {
        recorder.save_everything();
    }
}

/// Borra todo el historial: lo guardado y lo que todavía no se guardó.
pub fn clear() -> Result<(), String> {
    match RECORDER.get() {
        Some(recorder) => recorder.clear(),
        None => Ok(()),
    }
}

/// Cómo se muestra una aplicación: su nombre y su icono del tema.
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct AppInfo {
    pub name: String,
    pub icon: String,
}

/// Lo que pide el tablero: los días de un rango, con lo que hace falta para
/// dibujarlos.
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ScreenTimeRange {
    pub enabled: bool,
    /// Hoy, en la zona horaria de la sesión: el tablero no adivina el día.
    pub today: NaiveDate,
    /// El primer día con algo guardado. Antes de ése no hay historia, y el
    /// promedio no cuenta esos días como cero.
    pub first_day: Option<NaiveDate>,
    /// Milisegundos por día y por `app-id`. Los días sin nada no vienen.
    pub days: BTreeMap<NaiveDate, BTreeMap<String, u64>>,
    pub apps: BTreeMap<String, AppInfo>,
}

fn app_info(app_id: &str) -> AppInfo {
    let icon =
        app_icon::icon_for_app_id(app_id).unwrap_or_else(|| app_icon::fallback_icon_name(app_id));
    let icon = if icon.is_empty() {
        app_icon::FALLBACK_ICON.to_string()
    } else {
        icon
    };
    AppInfo {
        name: app_icon::name_for_app_id(app_id).unwrap_or_else(|| app_id.to_string()),
        icon,
    }
}

/// El rango que se pide no puede ser más largo que esto: el tablero pide un
/// mes y una semana, y un pedido de años sería leer miles de archivos.
const MAX_RANGE_DAYS: i64 = 62;

fn parse_range(from: &str, to: &str) -> Result<(NaiveDate, NaiveDate), String> {
    let parse = |text: &str| {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|_| format!("fecha inválida: «{text}»"))
    };
    let (from, to) = (parse(from)?, parse(to)?);
    if from > to {
        return Err("el rango termina antes de empezar".into());
    }
    if (to - from).num_days() > MAX_RANGE_DAYS {
        return Err(format!("el rango no puede pasar de {MAX_RANGE_DAYS} días"));
    }
    Ok((from, to))
}

/// Los días de `from` a `to` (`AAAA-MM-DD`, los dos incluidos).
#[tauri::command]
pub fn screen_time_range(from: String, to: String) -> Result<ScreenTimeRange, String> {
    let (from, to) = parse_range(&from, &to)?;
    let Some(recorder) = RECORDER.get() else {
        return Err("el registro de tiempo de pantalla no arrancó".into());
    };
    let days = recorder.usage(from, to);
    let apps = days
        .values()
        .flat_map(|apps| apps.keys())
        .map(|app| (app.clone(), app_info(app)))
        .collect();
    Ok(ScreenTimeRange {
        enabled: recorder.enabled.load(Ordering::SeqCst),
        today: Local::now().date_naive(),
        first_day: recorder.store.as_ref().and_then(Store::first_day),
        days,
        apps,
    })
}

/// «Borrar historial».
#[tauri::command]
pub fn screen_time_clear() -> Result<(), String> {
    clear()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prendido_salvo_que_la_configuración_diga_que_no() {
        assert!(enabled_from_json("{}"));
        assert!(enabled_from_json("no es json"));
        assert!(enabled_from_json(r#"{"screen_time":{}}"#));
        assert!(enabled_from_json(r#"{"screen_time":{"enabled":"no"}}"#));
        assert!(enabled_from_json(r#"{"screen_time":{"enabled":true}}"#));
        assert!(!enabled_from_json(r#"{"screen_time":{"enabled":false}}"#));
    }

    #[test]
    fn el_rango_se_valida_antes_de_leer_nada() {
        assert!(parse_range("2026-03-16", "2026-03-22").is_ok());
        assert!(parse_range("2026-03-16", "2026-03-16").is_ok());
        assert!(parse_range("2026-03-22", "2026-03-16").is_err());
        assert!(parse_range("ayer", "2026-03-16").is_err());
        assert!(parse_range("2026-01-01", "2026-12-31").is_err());
        // Un mes con su semana de antes entra.
        assert!(parse_range("2026-02-23", "2026-03-31").is_ok());
    }

    #[test]
    fn una_aplicación_sin_entrada_se_muestra_con_su_app_id() {
        let info = app_info("vasak-app-que-no-existe-150");
        assert_eq!(info.name, "vasak-app-que-no-existe-150");
        assert_eq!(info.icon, "vasak-app-que-no-existe-150");
    }

    #[test]
    fn lo_que_viaja_al_tablero_tiene_las_fechas_como_texto() {
        let mut days = BTreeMap::new();
        days.insert(
            NaiveDate::from_ymd_opt(2026, 3, 16).expect("fecha"),
            BTreeMap::from([("firefox".to_string(), 1000_u64)]),
        );
        let range = ScreenTimeRange {
            enabled: true,
            today: NaiveDate::from_ymd_opt(2026, 3, 16).expect("fecha"),
            first_day: None,
            days,
            apps: BTreeMap::new(),
        };
        let json = serde_json::to_value(&range).expect("se serializa");
        assert_eq!(json["today"], "2026-03-16");
        assert_eq!(json["days"]["2026-03-16"]["firefox"], 1000);
        assert!(json["first_day"].is_null());
    }

    /// El recorrido entero, con un directorio temporal: lo que entrega el
    /// tracker llega al disco y vuelve en el rango, sin contarse dos veces.
    #[test]
    fn lo_guardado_y_lo_vivo_se_suman_sin_repetir() {
        let temp = store::tests::TempDir::new("recorder");
        let recorder = Recorder {
            tracker: Mutex::new(Tracker::new(
                Local,
                ChronoDuration::minutes(3),
                ChronoDuration::seconds(30),
            )),
            store: Some(Store::new(temp.0.clone())),
            unsaved: Mutex::new(Usage::new()),
            enabled: AtomicBool::new(true),
        };
        let start = Utc::now() - ChronoDuration::minutes(10);
        let mut now = start;
        while now <= Utc::now() {
            recorder.tracker.lock().tick(now, Some("firefox"), false);
            now += ChronoDuration::seconds(2);
        }
        recorder.save_finalized();
        let today = Local::now().date_naive();
        let yesterday = today.pred_opt().expect("ayer");

        let total: u64 = recorder
            .usage(yesterday, today)
            .values()
            .filter_map(|apps| apps.get("firefox"))
            .sum();
        let expected = (Utc::now() - start).num_milliseconds() as u64;
        // Lo que tardó la prueba en correr es la diferencia; diez minutos con
        // una tolerancia de cinco segundos.
        assert!(total.abs_diff(expected) < 5_000, "{total} vs {expected}");
        assert!(std::fs::read_dir(&temp.0).expect("se lista").count() >= 1);

        recorder.clear().expect("se borra");
        assert!(recorder.usage(yesterday, today).is_empty());
        assert_eq!(std::fs::read_dir(&temp.0).expect("se lista").count(), 0);
    }
}
