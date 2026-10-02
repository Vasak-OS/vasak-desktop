//! La contabilidad del tiempo de pantalla, sin nada alrededor.
//!
//! Recibe lo que pasa —qué aplicación tiene el foco en cada vuelta, si la
//! pantalla está bloqueada, cuándo empezó y terminó la inactividad— y devuelve
//! cuánto tiempo va a cada aplicación en cada día. No lee el reloj, ni el
//! compositor, ni el disco: todo eso entra por parámetro, y por eso se puede
//! probar entera con horas inventadas (`tests` al final).
//!
//! # Qué cuenta
//!
//! El tiempo con una ventana de aplicación enfocada, con la pantalla sin
//! bloquear y sin inactividad. Las tres condiciones a la vez: una ventana
//! enfocada detrás de la pantalla de bloqueo no es tiempo de uso, y tampoco lo
//! es la que quedó enfocada mientras la persona se fue a tomar un café.
//!
//! # La inactividad llega tarde, y por eso hay un tramo pendiente
//!
//! El compositor avisa la inactividad **después** del umbral: si el umbral es
//! de tres minutos, el aviso llega tres minutos después del último movimiento,
//! y esos tres minutos ya se contaron. Lo justo es descontarlos. Para poder
//! hacerlo sin reescribir lo ya guardado, el tiempo de los últimos `window`
//! segundos queda **pendiente**: se ve en el tablero, pero no se guarda hasta
//! que ya no pueda caer dentro de una inactividad. Si no hubo un aviso en
//! `window` segundos, el último movimiento fue hace menos que eso, así que
//! nada anterior a `ahora - window` puede descontarse: eso es lo que se
//! entrega para guardar ([`Tracker::finalize`]).
//!
//! # Medianoche
//!
//! Lo que cruza la medianoche se reparte entre los dos días, en la zona
//! horaria que se le pase: una hora de 23:30 a 00:30 son treinta minutos para
//! cada día. Con cambio de horario la medianoche local puede no existir (en
//! algunas zonas el reloj salta de 23:59 a 01:00); ahí el día empieza en el
//! primer instante que sí existe.
//!
//! # Saltos del reloj
//!
//! Si entre dos vueltas pasó mucho más de lo esperado —una suspensión sin
//! bloqueo, el proceso detenido—, lo del medio no se cuenta: el tramo se
//! cierra en la última vuelta vista. Si el reloj fue para atrás, lo mismo.

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use std::collections::BTreeMap;

/// Milisegundos por día y por aplicación.
pub type Usage = BTreeMap<NaiveDate, BTreeMap<String, u64>>;

/// Un tramo de tiempo de una aplicación.
#[derive(Debug, Clone, PartialEq)]
struct Segment {
    app: String,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

/// La aplicación que tiene el foco desde `start`, todavía sin cerrar.
#[derive(Debug, Clone, PartialEq)]
struct Open {
    app: String,
    start: DateTime<Utc>,
}

pub struct Tracker<Tz: TimeZone> {
    tz: Tz,
    /// Lo que todavía puede descontarse: ver la documentación del módulo.
    window: Duration,
    /// Más que esto entre dos vueltas es un salto del reloj.
    max_gap: Duration,
    open: Option<Open>,
    pending: Vec<Segment>,
    idle: bool,
    last_tick: Option<DateTime<Utc>>,
}

impl<Tz: TimeZone> Tracker<Tz> {
    pub fn new(tz: Tz, window: Duration, max_gap: Duration) -> Self {
        Self {
            tz,
            window,
            max_gap,
            open: None,
            pending: Vec::new(),
            idle: false,
            last_tick: None,
        }
    }

    /// Una vuelta: a esta hora, ésta es la aplicación enfocada y así está el
    /// bloqueo.
    ///
    /// `focused` es el `app-id` de la ventana de aplicación activa, o `None`
    /// si no hay (el escritorio, un applet, nada abierto).
    pub fn tick(&mut self, now: DateTime<Utc>, focused: Option<&str>, locked: bool) {
        if let Some(last) = self.last_tick {
            if now < last || now - last > self.max_gap {
                // Lo del medio no se vio: no se cuenta.
                self.close_at(last);
            }
        }
        self.last_tick = Some(now);

        let app = focused
            .map(str::trim)
            .filter(|app| !app.is_empty())
            .filter(|_| !locked && !self.idle);

        match (app, self.open.as_ref()) {
            (Some(app), Some(open)) if open.app == app => {}
            (Some(app), _) => {
                self.close_at(now);
                self.open = Some(Open {
                    app: app.to_string(),
                    start: now,
                });
            }
            (None, _) => self.close_at(now),
        }
    }

    /// El compositor avisó la inactividad a las `now`; la persona dejó de
    /// tocar a las `since` (`now` menos el umbral). Lo de después de `since`
    /// se descuenta.
    pub fn idle_started(&mut self, now: DateTime<Utc>, since: DateTime<Utc>) {
        self.idle = true;
        self.close_at(now);

        let since = since.min(now);
        self.pending.retain_mut(|segment| {
            if segment.start >= since {
                return false;
            }
            segment.end = segment.end.min(since);
            true
        });
    }

    /// Volvió la actividad. La aplicación enfocada vuelve a contar desde la
    /// próxima vuelta.
    pub fn idle_ended(&mut self) {
        self.idle = false;
    }

    #[cfg(test)]
    pub fn is_idle(&self) -> bool {
        self.idle
    }

    /// Lo que ya no puede descontarse, para guardarlo. Sale del tracker: la
    /// próxima llamada no lo vuelve a dar.
    pub fn finalize(&mut self, now: DateTime<Utc>) -> Usage {
        let safe = now - self.window;
        let mut out = Usage::new();

        // Lo abierto se parte en lo seguro y lo que sigue: así un tramo largo
        // de una sola aplicación también se va guardando.
        if let Some(open) = self.open.as_mut() {
            if open.start < safe {
                self.pending.push(Segment {
                    app: open.app.clone(),
                    start: open.start,
                    end: safe,
                });
                open.start = safe;
            }
        }

        let mut kept = Vec::new();
        for segment in self.pending.drain(..) {
            if segment.end <= safe {
                add_split(&mut out, &self.tz, &segment.app, segment.start, segment.end);
            } else if segment.start < safe {
                add_split(&mut out, &self.tz, &segment.app, segment.start, safe);
                kept.push(Segment {
                    start: safe,
                    ..segment
                });
            } else {
                kept.push(segment);
            }
        }
        self.pending = kept;
        out
    }

    /// Todo lo contado, sin esperar a la ventana de descuento: para cuando el
    /// proceso se va o se apaga el registro. Lo abierto se cierra a las `now`.
    pub fn finalize_all(&mut self, now: DateTime<Utc>) -> Usage {
        self.close_at(now);
        let mut out = Usage::new();
        for segment in self.pending.drain(..) {
            add_split(&mut out, &self.tz, &segment.app, segment.start, segment.end);
        }
        out
    }

    /// Lo contado que todavía no se entregó, hasta las `now`: lo que el tablero
    /// suma a lo guardado para mostrar el día al segundo.
    pub fn live(&self, now: DateTime<Utc>) -> Usage {
        let mut out = Usage::new();
        for segment in &self.pending {
            add_split(&mut out, &self.tz, &segment.app, segment.start, segment.end);
        }
        if let Some(open) = &self.open {
            add_split(&mut out, &self.tz, &open.app, open.start, now);
        }
        out
    }

    /// Olvida lo que no se entregó: para «borrar historial».
    pub fn discard(&mut self) {
        self.open = None;
        self.pending.clear();
    }

    fn close_at(&mut self, at: DateTime<Utc>) {
        if let Some(open) = self.open.take() {
            if at > open.start {
                self.pending.push(Segment {
                    app: open.app,
                    start: open.start,
                    end: at,
                });
            }
        }
    }
}

/// Suma `start..end` a `app`, repartido entre los días locales que toca.
fn add_split<Tz: TimeZone>(
    out: &mut Usage,
    tz: &Tz,
    app: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) {
    let mut cursor = start;
    while cursor < end {
        let day = cursor.with_timezone(tz).date_naive();
        let next = start_of_day(tz, day + Duration::days(1)).unwrap_or(end);
        let until = next.min(end).max(cursor);
        let ms = (until - cursor).num_milliseconds();
        if ms > 0 {
            *out.entry(day)
                .or_default()
                .entry(app.to_string())
                .or_default() += ms as u64;
        }
        if until <= cursor {
            break;
        }
        cursor = until;
    }
}

/// El primer instante del día `day` en `tz`. Si la medianoche no existe por un
/// cambio de horario, la primera hora que sí.
fn start_of_day<Tz: TimeZone>(tz: &Tz, day: NaiveDate) -> Option<DateTime<Utc>> {
    (0..24).find_map(|hour| {
        let naive = day.and_hms_opt(hour, 0, 0)?;
        tz.from_local_datetime(&naive)
            .earliest()
            .map(|local| local.with_timezone(&Utc))
    })
}

/// Junta dos registros sumando lo de cada aplicación en cada día.
pub fn merge(into: &mut Usage, more: &Usage) {
    for (day, apps) in more {
        let target = into.entry(*day).or_default();
        for (app, ms) in apps {
            *target.entry(app.clone()).or_default() += ms;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    /// Buenos Aires: UTC−3, sin horario de verano.
    fn tz() -> FixedOffset {
        FixedOffset::west_opt(3 * 3600).expect("zona válida")
    }

    /// Una hora local de Buenos Aires, como instante.
    fn at(day: u32, hour: u32, minute: u32, second: u32) -> DateTime<Utc> {
        tz().with_ymd_and_hms(2026, 3, day, hour, minute, second)
            .single()
            .expect("hora válida")
            .with_timezone(&Utc)
    }

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, day).expect("fecha válida")
    }

    fn tracker() -> Tracker<FixedOffset> {
        Tracker::new(tz(), Duration::minutes(3), Duration::seconds(30))
    }

    /// Avanza de a `step` segundos desde `from` hasta `to`, con `focused`.
    fn run(
        tracker: &mut Tracker<FixedOffset>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        focused: Option<&str>,
        locked: bool,
    ) {
        let mut now = from;
        while now <= to {
            tracker.tick(now, focused, locked);
            now += Duration::seconds(2);
        }
    }

    fn ms(usage: &Usage, day: u32, app: &str) -> u64 {
        usage
            .get(&date(day))
            .and_then(|apps| apps.get(app))
            .copied()
            .unwrap_or(0)
    }

    #[test]
    fn el_cambio_de_foco_reparte_el_tiempo_entre_las_dos_aplicaciones() {
        let mut t = tracker();
        run(
            &mut t,
            at(16, 10, 0, 0),
            at(16, 10, 10, 0),
            Some("firefox"),
            false,
        );
        run(
            &mut t,
            at(16, 10, 10, 2),
            at(16, 10, 15, 0),
            Some("kitty"),
            false,
        );

        let all = t.finalize_all(at(16, 10, 15, 0));
        // Firefox hasta que llegó la vuelta de kitty, 10:10:02.
        assert_eq!(ms(&all, 16, "firefox"), (10 * 60 + 2) * 1000);
        assert_eq!(ms(&all, 16, "kitty"), (4 * 60 + 58) * 1000);
    }

    #[test]
    fn sin_ventana_enfocada_no_cuenta_nada() {
        let mut t = tracker();
        run(&mut t, at(16, 10, 0, 0), at(16, 10, 5, 0), None, false);
        run(
            &mut t,
            at(16, 10, 5, 2),
            at(16, 10, 6, 0),
            Some("  "),
            false,
        );

        assert!(t.finalize_all(at(16, 10, 6, 0)).is_empty());
    }

    #[test]
    fn con_la_pantalla_bloqueada_no_cuenta_aunque_la_ventana_siga_enfocada() {
        let mut t = tracker();
        run(
            &mut t,
            at(16, 9, 0, 0),
            at(16, 9, 30, 0),
            Some("firefox"),
            false,
        );
        // Bloqueo de una hora: Firefox sigue «activado» para el compositor.
        run(
            &mut t,
            at(16, 9, 30, 2),
            at(16, 10, 30, 0),
            Some("firefox"),
            true,
        );
        run(
            &mut t,
            at(16, 10, 30, 2),
            at(16, 10, 40, 0),
            Some("firefox"),
            false,
        );

        let all = t.finalize_all(at(16, 10, 40, 0));
        assert_eq!(ms(&all, 16, "firefox"), (30 * 60 + 2 + 9 * 60 + 58) * 1000);
    }

    #[test]
    fn la_inactividad_descuenta_lo_que_pasó_desde_el_último_movimiento() {
        let mut t = tracker();
        // Trabaja hasta las 11:00 y se va; el aviso llega tres minutos después.
        run(
            &mut t,
            at(16, 10, 0, 0),
            at(16, 11, 3, 0),
            Some("telegram"),
            false,
        );
        t.idle_started(at(16, 11, 3, 0), at(16, 11, 0, 0));
        // Mientras no hay nadie, la ventana sigue enfocada.
        run(
            &mut t,
            at(16, 11, 3, 2),
            at(16, 11, 30, 0),
            Some("telegram"),
            false,
        );
        t.idle_ended();
        run(
            &mut t,
            at(16, 11, 30, 2),
            at(16, 11, 40, 2),
            Some("telegram"),
            false,
        );

        let all = t.finalize_all(at(16, 11, 40, 2));
        assert_eq!(ms(&all, 16, "telegram"), (60 * 60 + 10 * 60) * 1000);
        assert!(!t.is_idle());
    }

    #[test]
    fn la_inactividad_también_descuenta_lo_que_ya_estaba_cerrado_en_el_pendiente() {
        let mut t = tracker();
        run(
            &mut t,
            at(16, 10, 0, 0),
            at(16, 10, 59, 0),
            Some("firefox"),
            false,
        );
        // A las 10:59 pasa a kitty sin tocar nada más (una ventana que se cerró).
        run(
            &mut t,
            at(16, 10, 59, 2),
            at(16, 11, 2, 0),
            Some("kitty"),
            false,
        );
        t.idle_started(at(16, 11, 2, 0), at(16, 10, 58, 0));

        let all = t.finalize_all(at(16, 11, 2, 0));
        assert_eq!(ms(&all, 16, "firefox"), 58 * 60 * 1000);
        assert_eq!(ms(&all, 16, "kitty"), 0);
    }

    #[test]
    fn lo_que_cruza_la_medianoche_se_reparte_entre_los_dos_días() {
        let mut t = tracker();
        run(
            &mut t,
            at(16, 23, 30, 0),
            at(16, 23, 59, 58),
            Some("kitty"),
            false,
        );
        run(
            &mut t,
            at(17, 0, 0, 0),
            at(17, 0, 30, 0),
            Some("kitty"),
            false,
        );

        let all = t.finalize_all(at(17, 0, 30, 0));
        assert_eq!(ms(&all, 16, "kitty"), 30 * 60 * 1000);
        assert_eq!(ms(&all, 17, "kitty"), 30 * 60 * 1000);
    }

    #[test]
    fn la_medianoche_es_la_local_y_no_la_de_utc() {
        // 21:00 en Buenos Aires es medianoche UTC: no parte nada.
        let mut t = tracker();
        run(
            &mut t,
            at(16, 20, 30, 0),
            at(16, 21, 30, 0),
            Some("firefox"),
            false,
        );

        let all = t.finalize_all(at(16, 21, 30, 0));
        assert_eq!(all.len(), 1);
        assert_eq!(ms(&all, 16, "firefox"), 60 * 60 * 1000);
    }

    #[test]
    fn un_salto_del_reloj_no_se_cuenta() {
        let mut t = tracker();
        run(
            &mut t,
            at(16, 10, 0, 0),
            at(16, 10, 10, 0),
            Some("firefox"),
            false,
        );
        // Suspendida sin bloqueo dos horas: la vuelta siguiente llega a las 12.
        run(
            &mut t,
            at(16, 12, 10, 0),
            at(16, 12, 20, 0),
            Some("firefox"),
            false,
        );

        let all = t.finalize_all(at(16, 12, 20, 0));
        assert_eq!(ms(&all, 16, "firefox"), 20 * 60 * 1000);
    }

    #[test]
    fn un_reloj_que_vuelve_atrás_tampoco() {
        let mut t = tracker();
        run(
            &mut t,
            at(16, 10, 0, 0),
            at(16, 10, 10, 0),
            Some("firefox"),
            false,
        );
        run(
            &mut t,
            at(16, 9, 0, 0),
            at(16, 9, 5, 0),
            Some("firefox"),
            false,
        );

        let all = t.finalize_all(at(16, 9, 5, 0));
        assert_eq!(ms(&all, 16, "firefox"), 15 * 60 * 1000);
    }

    #[test]
    fn se_guarda_sólo_lo_que_ya_no_puede_descontarse() {
        let mut t = tracker();
        run(
            &mut t,
            at(16, 10, 0, 0),
            at(16, 10, 10, 0),
            Some("firefox"),
            false,
        );

        // A las 10:10, con tres minutos de ventana, es seguro hasta las 10:07.
        let saved = t.finalize(at(16, 10, 10, 0));
        assert_eq!(ms(&saved, 16, "firefox"), 7 * 60 * 1000);
        // Y lo que se entregó no se vuelve a entregar.
        assert!(t.finalize(at(16, 10, 10, 0)).is_empty());
        // Lo vivo es lo que falta guardar: los tres minutos.
        assert_eq!(ms(&t.live(at(16, 10, 10, 0)), 16, "firefox"), 3 * 60 * 1000);

        // Lo guardado más lo que queda da el total, sin contar dos veces.
        let rest = t.finalize_all(at(16, 10, 10, 0));
        assert_eq!(
            ms(&saved, 16, "firefox") + ms(&rest, 16, "firefox"),
            10 * 60 * 1000
        );
    }

    #[test]
    fn lo_vivo_incluye_lo_abierto_hasta_ahora_sin_consumirlo() {
        let mut t = tracker();
        run(
            &mut t,
            at(16, 10, 0, 0),
            at(16, 10, 1, 0),
            Some("firefox"),
            false,
        );

        assert_eq!(ms(&t.live(at(16, 10, 1, 30)), 16, "firefox"), 90 * 1000);
        assert_eq!(ms(&t.live(at(16, 10, 1, 30)), 16, "firefox"), 90 * 1000);
    }

    #[test]
    fn descartar_olvida_lo_pendiente_y_lo_abierto() {
        let mut t = tracker();
        run(
            &mut t,
            at(16, 10, 0, 0),
            at(16, 10, 1, 0),
            Some("firefox"),
            false,
        );
        t.discard();

        assert!(t.live(at(16, 10, 2, 0)).is_empty());
        assert!(t.finalize_all(at(16, 10, 2, 0)).is_empty());
    }

    #[test]
    fn juntar_suma_por_día_y_por_aplicación() {
        let mut a = Usage::new();
        a.entry(date(1)).or_default().insert("x".into(), 5);
        let mut b = Usage::new();
        b.entry(date(1)).or_default().insert("x".into(), 7);
        b.entry(date(2)).or_default().insert("y".into(), 1);

        merge(&mut a, &b);
        assert_eq!(a[&date(1)]["x"], 12);
        assert_eq!(a[&date(2)]["y"], 1);
    }

    #[test]
    fn un_tramo_de_varios_días_se_reparte_entero_sin_perder_nada() {
        let mut out = Usage::new();
        add_split(&mut out, &tz(), "kitty", at(16, 22, 0, 0), at(18, 2, 0, 0));
        let total: u64 = out.values().flat_map(|apps| apps.values()).sum();
        assert_eq!(total, 28 * 3600 * 1000);
        assert_eq!(ms(&out, 17, "kitty"), 24 * 3600 * 1000);
    }
}
