//! El tiempo de pantalla por aplicación (vasak-desktop#150).
//!
//! Cuánto tiempo estuvo enfocada cada aplicación, por día, para el tablero que
//! abre el centro de control. **La contabilidad ya no vive acá**: se mudó al
//! servicio `vasak-health-service`. El escritorio hace sólo lo que únicamente él
//! puede hacer —ver el compositor (la ventana enfocada por el socket de Wayfire)
//! y la inactividad (`ext-idle-notify-v1`)— y **empuja** cada muestra al
//! servicio por D-Bus; cuando el tablero pide el informe, se lo **consulta** y
//! lo **enriquece** con el nombre y el icono de cada aplicación.
//!
//! - [`activity`]: el sensado del bloqueo (por `/proc`) y la inactividad
//!   (`ext-idle-notify-v1`, tres minutos). Se queda en el escritorio.
//! - [`health`]: el cliente D-Bus del servicio (los tipos espejo del contrato y
//!   las llamadas por el pool compartido).
//!
//! # Cómo corre
//!
//! Un hilo da una vuelta cada [`TICK`]: pregunta al compositor qué ventana está
//! enfocada y si hay un programa de bloqueo, y lo empuja con `PushSample`. Los
//! avisos de inactividad llegan por un canal desde el hilo de Wayland y se
//! reenvían en el momento (`IdleStarted`/`IdleEnded`). Si el canal se cae, se
//! avisa una vez con `IdleMonitorLost`. Al cerrar la sesión desde el escritorio
//! (`logout`, `shutdown`, `reboot`) se manda [`flush`].
//!
//! # Privacidad
//!
//! Nada sale de la máquina: el servicio guarda los archivos del usuario y nadie
//! más los lee. Se apaga con `screen_time.enabled: false` en `vasak.conf` —lo
//! escribe el interruptor del tablero, y lo va a escribir Configuración—; el
//! escritorio sigue ese cambio por `config-changed` y se lo pasa al servicio con
//! `SetEnabled`. Apagado no se empuja nada. Se borra con «Borrar historial»
//! ([`clear`], también por D-Bus desde Configuración).

pub mod activity;
pub mod health;

use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Listener};

use crate::logger::{log_error, log_info};
use crate::window_manager::app_icon;
use activity::{IdleEvent, IDLE_THRESHOLD};

/// Cada cuánto se mira el foco. El error de un cambio de aplicación es, como
/// mucho, esto.
pub const TICK: Duration = Duration::from_secs(2);

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

/// Arranca el sensado-que-empuja. Se llama una vez, desde el `setup`.
pub fn start(app: &AppHandle) {
    let enabled = Arc::new(AtomicBool::new(read_enabled()));

    // Prender o apagar desde la configuración, en el acto: se actualiza el filtro
    // local (apagado no se empuja nada) y se le avisa al servicio, que además
    // guarda lo contado al apagarse.
    {
        let closure_app = app.clone();
        let enabled = enabled.clone();
        app.listen("config-changed", move |_| {
            let now = read_enabled();
            let was = enabled.swap(now, Ordering::SeqCst);
            if was != now {
                log_info(if now {
                    "[tiempo de pantalla] prendido desde la configuración"
                } else {
                    "[tiempo de pantalla] apagado desde la configuración"
                });
                let app = closure_app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = health::set_enabled(&app, now).await {
                        log_error(&format!(
                            "[tiempo de pantalla] no se pudo avisar al servicio del cambio: {error}"
                        ));
                    }
                });
            }
        });
    }

    let (tx, rx) = channel();
    activity::watch_idle(tx);
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("screen-time".into())
        .spawn(move || run(app, enabled, rx));
    if let Err(error) = spawned {
        log_error(&format!(
            "[tiempo de pantalla] no se pudo arrancar: {error}"
        ));
    }
}

fn run(app: AppHandle, enabled: Arc<AtomicBool>, idle: Receiver<IdleEvent>) {
    let mut next_tick = Instant::now();
    let mut idle_monitor = true;
    // Para no llenar el registro cuando el servicio no está: se avisa en el
    // cambio de estado (anduvo → falló, falló → anduvo), no en cada muestra.
    let mut healthy = true;
    loop {
        let wait = next_tick.saturating_duration_since(Instant::now());
        match idle.recv_timeout(wait) {
            Ok(event) => {
                on_idle(&app, event);
                continue;
            }
            Err(RecvTimeoutError::Timeout) => {}
            // Sin hilo de inactividad (el compositor no ofrece el protocolo, o
            // la conexión de Wayland se cayó): se le avisa al servicio una vez
            // para que deje de contar y no sume tiempo sin nadie adelante.
            Err(RecvTimeoutError::Disconnected) => {
                if idle_monitor {
                    idle_monitor = false;
                    let _ = push(health::idle_monitor_lost(&app));
                }
                std::thread::sleep(wait);
            }
        }
        next_tick = Instant::now() + TICK;

        if enabled.load(Ordering::SeqCst) {
            let app_id = focused_app().unwrap_or_default();
            let locked = activity::screen_locked();
            let result = push(health::push_sample(&app, &app_id, locked));
            report_health(&mut healthy, result);
        }
    }
}

/// Reenvía un evento de inactividad al servicio.
fn on_idle(app: &AppHandle, event: IdleEvent) {
    match event {
        // El aviso del compositor llega recién pasado el umbral, así que la
        // inactividad empezó hace `IDLE_THRESHOLD`: eso se descuenta en el
        // servicio.
        IdleEvent::Idle => {
            let idle_ms = u64::try_from(IDLE_THRESHOLD.as_millis()).unwrap_or(u64::MAX);
            let _ = push(health::idle_started(app, idle_ms));
        }
        IdleEvent::Active => {
            let _ = push(health::idle_ended(app));
        }
    }
}

/// Corre una llamada asíncrona al servicio desde el hilo de sensado. Igual que
/// `focused_app`, bloquea esperando la respuesta: las llamadas son locales y
/// baratas, y el hilo no toca el bucle principal de GTK.
fn push(future: impl std::future::Future<Output = Result<(), String>>) -> Result<(), String> {
    tauri::async_runtime::block_on(future)
}

/// Anota sólo el cambio de estado, para no repetir el mismo error cada dos
/// segundos cuando el servicio no está.
fn report_health(healthy: &mut bool, result: Result<(), String>) {
    match result {
        Ok(()) => {
            if !*healthy {
                *healthy = true;
                log_info("[tiempo de pantalla] el servicio volvió a responder");
            }
        }
        Err(error) => {
            if *healthy {
                *healthy = false;
                log_error(&format!(
                    "[tiempo de pantalla] no se pudo empujar la muestra al servicio: {error}"
                ));
            }
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

/// Cuánto se espera a que el servicio guarde antes de cerrar la sesión o apagar.
///
/// Corto a propósito: esto corre en el camino de `logout`/`shutdown`/`reboot`, y
/// una llamada de D-Bus que se cuelga —el servicio trabado, o la activación por
/// D-Bus que no termina— esperaría si no el tiempo por omisión de zbus (~25 s),
/// demorando la acción de energía todo ese rato. Guardar es deseable, no al
/// precio de dejar la pantalla congelada al apagar.
const FLUSH_TIMEOUT: Duration = Duration::from_secs(2);

/// Guarda todo lo contado, sin esperar a la ventana de descuento: antes de
/// cerrar la sesión o apagar. Si el servicio no contesta en [`FLUSH_TIMEOUT`],
/// se sigue igual.
pub async fn flush(app: &AppHandle) {
    flush_within(FLUSH_TIMEOUT, health::flush(app)).await;
}

/// Espera a que `save` termine, pero no más de `timeout`, y anota lo que pase.
/// Separa el «cuánto esperar y qué anotar» de la llamada de D-Bus para poder
/// probarlo sin un bus.
async fn flush_within(
    timeout: Duration,
    save: impl std::future::Future<Output = Result<(), String>>,
) {
    match tokio::time::timeout(timeout, save).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => log_error(&format!(
            "[tiempo de pantalla] no se pudo guardar antes de cerrar: {error}"
        )),
        Err(_) => log_error(
            "[tiempo de pantalla] el servicio no contestó a tiempo al guardar antes de cerrar",
        ),
    }
}

/// Borra todo el historial.
pub async fn clear(app: &AppHandle) -> Result<(), String> {
    health::clear_screen_time(app).await
}

/// Cómo se muestra una aplicación: su nombre, su icono del tema y su categoría.
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct AppInfo {
    pub name: String,
    pub icon: String,
    /// La categoría freedesktop principal que trae el servicio, para los
    /// informes por categoría. Vacía si no se pudo averiguar.
    pub category: String,
}

/// Lo que pide el tablero: los días de un rango, con lo que hace falta para
/// dibujarlos.
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ScreenTimeRange {
    pub enabled: bool,
    /// Hoy, en la zona horaria de la sesión: el tablero no adivina el día.
    pub today: String,
    /// El primer día con algo guardado. Antes de ése no hay historia, y el
    /// promedio no cuenta esos días como cero.
    pub first_day: Option<String>,
    /// Milisegundos por día y por `app-id`. Los días sin nada no vienen. Es la
    /// forma que el tablero ya dibuja; no cambió.
    pub days: BTreeMap<String, BTreeMap<String, u64>>,
    pub apps: BTreeMap<String, AppInfo>,
    /// Milisegundos por hora (24 valores) por día y por `app-id`, para los
    /// informes nuevos de horario. Los días sin desglose no vienen.
    pub hours: BTreeMap<String, BTreeMap<String, Vec<u64>>>,
}

fn app_info(app_id: &str, category: String) -> AppInfo {
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
        category,
    }
}

/// Arma lo que viaja al tablero a partir del informe del servicio: separa el
/// total por día (lo que el tablero ya dibuja) del desglose por hora, y
/// enriquece cada `app-id` con su nombre y su icono del tema.
fn build_range(report: health::ScreenTimeReport) -> ScreenTimeRange {
    let mut days: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    let mut hours: BTreeMap<String, BTreeMap<String, Vec<u64>>> = BTreeMap::new();
    let mut apps: BTreeMap<String, AppInfo> = BTreeMap::new();

    for day in report.days {
        let mut totals = BTreeMap::new();
        let mut by_hour = BTreeMap::new();
        for app in day.apps {
            totals.insert(app.app_id.clone(), app.millis);
            // Sólo viaja el desglose bien formado y con algo: el contrato manda
            // siempre [`health::HOURS_IN_DAY`] valores, y los datos viejos traen
            // todo en cero (sólo el total).
            if app.hours.len() == health::HOURS_IN_DAY && app.hours.iter().any(|&ms| ms > 0) {
                by_hour.insert(app.app_id.clone(), app.hours);
            }
            apps.entry(app.app_id.clone())
                .or_insert_with(|| app_info(&app.app_id, app.category));
        }
        if !totals.is_empty() {
            days.insert(day.date.clone(), totals);
        }
        if !by_hour.is_empty() {
            hours.insert(day.date, by_hour);
        }
    }

    ScreenTimeRange {
        enabled: report.enabled,
        today: report.today,
        first_day: Some(report.first_day).filter(|day| !day.is_empty()),
        days,
        apps,
        hours,
    }
}

/// El rango que se pide no puede ser más largo que esto: el tablero pide un mes
/// y una semana, y un pedido de años sería leer miles de archivos. El servicio
/// valida igual, pero rechazar acá evita una vuelta al bus por una fecha mal
/// escrita.
const MAX_RANGE_DAYS: i64 = 62;

fn parse_range(from: &str, to: &str) -> Result<(), String> {
    use chrono::NaiveDate;
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
    Ok(())
}

/// Los días de `from` a `to` (`AAAA-MM-DD`, los dos incluidos).
#[tauri::command]
pub async fn screen_time_range(
    app: AppHandle,
    from: String,
    to: String,
) -> Result<ScreenTimeRange, String> {
    parse_range(&from, &to)?;
    let report = health::screen_time(&app, &from, &to).await?;
    Ok(build_range(report))
}

/// «Borrar historial».
#[tauri::command]
pub async fn screen_time_clear(app: AppHandle) -> Result<(), String> {
    clear(&app).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use health::{AppUsage, DayUsage, ScreenTimeReport};

    fn report() -> ScreenTimeReport {
        let mut morning = vec![0u64; health::HOURS_IN_DAY];
        morning[9] = 1_000;
        morning[10] = 2_000;
        ScreenTimeReport {
            enabled: true,
            today: "2026-03-16".into(),
            first_day: "2026-03-15".into(),
            days: vec![
                DayUsage {
                    date: "2026-03-15".into(),
                    apps: vec![AppUsage {
                        app_id: "firefox".into(),
                        category: "Network".into(),
                        millis: 3_000,
                        hours: morning,
                    }],
                },
                DayUsage {
                    date: "2026-03-16".into(),
                    apps: vec![
                        AppUsage {
                            app_id: "code".into(),
                            category: "Development".into(),
                            millis: 5_000,
                            // Sin desglose por hora (datos viejos): no viaja en
                            // `hours`, pero el total sí.
                            hours: vec![0u64; health::HOURS_IN_DAY],
                        },
                        AppUsage {
                            app_id: "firefox".into(),
                            category: "Network".into(),
                            millis: 1_000,
                            hours: vec![0u64; health::HOURS_IN_DAY],
                        },
                    ],
                },
            ],
        }
    }

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
    fn el_rango_se_valida_antes_de_ir_al_bus() {
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
        let info = app_info("vasak-app-que-no-existe-150", "Game".into());
        assert_eq!(info.name, "vasak-app-que-no-existe-150");
        assert_eq!(info.icon, "vasak-app-que-no-existe-150");
        assert_eq!(info.category, "Game");
    }

    #[test]
    fn el_informe_del_servicio_se_separa_en_total_y_horas_y_se_enriquece() {
        let range = build_range(report());

        // El total por día es lo que el tablero ya dibuja: día → app-id → ms.
        assert_eq!(range.days["2026-03-15"]["firefox"], 3_000);
        assert_eq!(range.days["2026-03-16"]["code"], 5_000);
        assert_eq!(range.days["2026-03-16"]["firefox"], 1_000);

        // El desglose por hora viaja aparte, y sólo cuando hay algo: el 15
        // tiene; el 16, todo en cero, no.
        assert_eq!(range.hours["2026-03-15"]["firefox"][9], 1_000);
        assert_eq!(range.hours["2026-03-15"]["firefox"][10], 2_000);
        assert!(!range.hours.contains_key("2026-03-16"));

        // Cada app queda enriquecida con su categoría una sola vez.
        assert_eq!(range.apps["firefox"].category, "Network");
        assert_eq!(range.apps["code"].category, "Development");
    }

    #[test]
    fn sin_historia_el_primer_día_es_none() {
        let mut empty = report();
        empty.first_day = String::new();
        empty.days.clear();
        let range = build_range(empty);
        assert_eq!(range.first_day, None);
        assert!(range.days.is_empty());
        assert!(range.hours.is_empty());
    }

    #[test]
    fn lo_que_viaja_al_tablero_tiene_las_fechas_como_texto() {
        let json = serde_json::to_value(build_range(report())).expect("se serializa");
        assert_eq!(json["today"], "2026-03-16");
        assert_eq!(json["first_day"], "2026-03-15");
        assert_eq!(json["days"]["2026-03-16"]["code"], 5_000);
        assert_eq!(json["apps"]["code"]["category"], "Development");
    }

    #[test]
    fn flush_se_rinde_si_el_servicio_no_contesta() {
        tauri::async_runtime::block_on(async {
            // El servicio colgado: un futuro que no termina nunca. `flush_within`
            // no se cuelga con él —corta al vencer el plazo— así la acción de
            // energía no espera los ~25 s por omisión de zbus.
            let start = Instant::now();
            flush_within(
                Duration::from_millis(20),
                std::future::pending::<Result<(), String>>(),
            )
            .await;
            let waited = start.elapsed();
            assert!(waited >= Duration::from_millis(20), "cortó antes del plazo");
            assert!(waited < Duration::from_secs(5), "esperó de más: {waited:?}");
        });
    }

    #[test]
    fn flush_vuelve_en_el_acto_si_ya_guardó() {
        tauri::async_runtime::block_on(async {
            // Guardó bien y rápido: no espera el plazo entero.
            let start = Instant::now();
            flush_within(Duration::from_secs(30), async { Ok(()) }).await;
            assert!(start.elapsed() < Duration::from_secs(1));
        });
    }

    #[test]
    fn el_cambio_de_salud_se_anota_una_vez() {
        let mut healthy = true;
        // Un error anota (pasa a no sano) y los siguientes no repiten.
        report_health(&mut healthy, Err("sin bus".into()));
        assert!(!healthy);
        report_health(&mut healthy, Err("sin bus".into()));
        assert!(!healthy);
        // La recuperación vuelve a sano.
        report_health(&mut healthy, Ok(()));
        assert!(healthy);
    }
}
