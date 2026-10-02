/**
 * Las cuentas del tablero de tiempo de pantalla (vasak-desktop#150).
 *
 * El backend (`src-tauri/src/screen_time/`) manda los milisegundos de cada
 * aplicación en cada día; todo lo que el tablero dice encima —el total del
 * día, el promedio de la semana, la diferencia con ayer, la parte de cada
 * aplicación— sale de acá, en funciones puras que se prueban sin ventana
 * (`screen-time.test.ts`).
 *
 * # Las fechas son texto
 *
 * Un día es `AAAA-MM-DD`, como lo manda el backend en la zona horaria de la
 * sesión. Las cuentas de días se hacen en UTC sobre ese texto, así que un
 * cambio de horario no corre un día (sumar 24 horas a una medianoche local
 * puede caer en el mismo día o saltear uno).
 *
 * # El promedio no inventa ceros
 *
 * Es el total de la semana dividido por los días de la semana que **ya
 * pasaron** (hasta hoy) y que **tienen historia** (desde el primer día
 * guardado). Un lunes, el promedio es el del lunes; la semana en que se
 * prendió el registro no cuenta como cero los días de antes. Un día que pasó
 * sin usar la computadora sí cuenta: eso es parte del promedio.
 */

/** Lo que manda `screen_time_range`. */
export interface ScreenTimeRange {
	enabled: boolean;
	today: string;
	first_day: string | null;
	/** Milisegundos por día y por `app-id`; los días sin nada no vienen. */
	days: Record<string, Record<string, number>>;
	apps: Record<string, { name: string; icon: string }>;
}

export interface AppUsage {
	appId: string;
	name: string;
	icon: string;
	ms: number;
	/** La parte de la que más se usó ese día, de 0 a 1. */
	share: number;
}

const DAY_MS = 86_400_000;

function toUtc(day: string): number {
	const [year, month, date] = day.split('-').map(Number);
	return Date.UTC(year ?? 1970, (month ?? 1) - 1, date ?? 1);
}

function fromUtc(time: number): string {
	return new Date(time).toISOString().slice(0, 10);
}

/** Si es un día bien escrito. */
export function isDay(text: unknown): text is string {
	return (
		typeof text === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(text) && fromUtc(toUtc(text)) === text
	);
}

export function addDays(day: string, amount: number): string {
	return fromUtc(toUtc(day) + amount * DAY_MS);
}

/** 0 lunes … 6 domingo. */
export function weekdayIndex(day: string): number {
	return (new Date(toUtc(day)).getUTCDay() + 6) % 7;
}

/** Los siete días de la semana de `day`, de lunes a domingo. */
export function weekOf(day: string): string[] {
	const monday = addDays(day, -weekdayIndex(day));
	return Array.from({ length: 7 }, (_, index) => addDays(monday, index));
}

/** El primero y el último día del mes de `day`. */
export function monthOf(day: string): { first: string; last: string; year: number; month: number } {
	const [year = 1970, month = 1] = day.split('-').map(Number);
	const first = fromUtc(Date.UTC(year, month - 1, 1));
	const last = fromUtc(Date.UTC(year, month, 0));
	return { first, last, year, month };
}

/** Lo que hay que pedirle al backend para dibujar el tablero de `day`. */
export function rangeFor(day: string): { from: string; to: string } {
	const week = weekOf(day);
	const month = monthOf(day);
	const starts = [month.first, addDays(day, -1)];
	const from = starts.reduce((a, b) => (a < b ? a : b), week[0] as string);
	const to = month.last > (week[6] as string) ? month.last : (week[6] as string);
	return { from, to };
}

/** El total de un día, en milisegundos. */
export function dayTotal(days: ScreenTimeRange['days'], day: string): number {
	const apps = days[day];
	if (!apps) return 0;
	return Object.values(apps).reduce((sum, ms) => sum + (Number.isFinite(ms) && ms > 0 ? ms : 0), 0);
}

/**
 * El promedio diario de la semana de `day`, o `null` si en esa semana todavía
 * no hay ningún día con historia que haya pasado.
 */
export function dailyAverage(
	days: ScreenTimeRange['days'],
	day: string,
	today: string,
	firstDay: string | null
): number | null {
	if (!firstDay) return null;
	const counted = weekOf(day).filter((each) => each <= today && each >= firstDay);
	if (counted.length === 0) return null;
	const total = counted.reduce((sum, each) => sum + dayTotal(days, each), 0);
	return total / counted.length;
}

/**
 * Cuánto más (o menos) que el día anterior. `null` si el día anterior es de
 * antes de que hubiera historia: comparar contra un día que no se midió diría
 * «subió todo».
 */
export function differenceFromYesterday(
	days: ScreenTimeRange['days'],
	day: string,
	firstDay: string | null
): number | null {
	const yesterday = addDays(day, -1);
	if (!firstDay || yesterday < firstDay) return null;
	return dayTotal(days, day) - dayTotal(days, yesterday);
}

/** Las aplicaciones de un día, de la más usada a la menos, con su parte. */
export function appsOf(range: Pick<ScreenTimeRange, 'days' | 'apps'>, day: string): AppUsage[] {
	const apps = range.days[day] ?? {};
	const rows = Object.entries(apps)
		.filter(([, ms]) => Number.isFinite(ms) && ms > 0)
		.map(([appId, ms]) => ({
			appId,
			name: range.apps[appId]?.name || appId,
			icon: range.apps[appId]?.icon || 'application-x-executable',
			ms,
		}))
		.sort((a, b) => b.ms - a.ms || a.name.localeCompare(b.name));
	const top = rows[0]?.ms ?? 0;
	return rows.map((row) => ({ ...row, share: top > 0 ? row.ms / top : 0 }));
}

/** Los textos con que se escribe una duración, del catálogo. */
export interface DurationWords {
	/** «{0} h {1} min» */
	hoursMinutes: string;
	/** «{0} h» */
	hours: string;
	/** «{0} min» */
	minutes: string;
	/** «menos de 1 min» */
	underMinute: string;
}

/**
 * Una duración como se lee: «4 h 13 min», «2 h», «40 min». Por debajo del
 * minuto, `underMinute` si hubo algo, y «0 min» si no hubo nada. Los minutos
 * se truncan: «1 h 59 min» no se redondea a «2 h» antes de llegar.
 */
export function formatDuration(ms: number, words: DurationWords): string {
	const safe = Number.isFinite(ms) && ms > 0 ? ms : 0;
	const totalMinutes = Math.floor(safe / 60_000);
	if (totalMinutes === 0) return safe > 0 ? words.underMinute : words.minutes.replace('{0}', '0');
	const hours = Math.floor(totalMinutes / 60);
	const minutes = totalMinutes % 60;
	if (hours === 0) return words.minutes.replace('{0}', String(minutes));
	if (minutes === 0) return words.hours.replace('{0}', String(hours));
	return words.hoursMinutes.replace('{0}', String(hours)).replace('{1}', String(minutes));
}

/** El día del mes (1–31) de `day`. */
export function dayOfMonth(day: string): number {
	return Number(day.slice(8, 10));
}

/** Los valores de un mes por día del mes, para el mapa de calor. */
export function monthValues(days: ScreenTimeRange['days'], day: string): Record<number, number> {
	const { first, last } = monthOf(day);
	const values: Record<number, number> = {};
	for (let each = first; each <= last; each = addDays(each, 1)) {
		const total = dayTotal(days, each);
		if (total > 0) values[dayOfMonth(each)] = total;
	}
	return values;
}

/** Si en el rango no hay nada de nada: el tablero vacío. */
export function isEmpty(days: ScreenTimeRange['days']): boolean {
	return Object.keys(days).every((day) => dayTotal(days, day) === 0);
}
