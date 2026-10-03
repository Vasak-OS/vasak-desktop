/**
 * Las cuentas del tablero de fecha (vasak-desktop#130), sin Vue.
 *
 * El tablero junta dos fuentes que ya tiene el escritorio —el clima de
 * `useWeather`, el mismo del widget y del menú, y los eventos del almacén
 * local de vasak-accounts (`calendar.service.ts`)— y las pasa a la forma que
 * piden los componentes de vue-libvasak: `CalendarEntry` para el calendario y
 * la lista, `ForecastHour` para el arco. Lo que se cuenta acá es lo que se
 * prueba: qué horas van en el arco, qué clima corresponde a un día y qué tan
 * lleno va cada anillo.
 */
import {
	addDays,
	type CalendarEntry,
	type ForecastHour,
	type IsoDate,
	type IsoMonth,
	monthGrid,
	parseIsoDate,
	toIsoDate,
} from '@vasakgroup/vue-libvasak';
import weatherCodes from '@/data/weatherCodes.json';
import type { CodeDataType } from '@/interfaces/weather';
import type { CalendarOccurrence } from '@/services/calendar.service';

// ── Los eventos ────────────────────────────────────────────────────────────

/** La clave de una vez de un evento: un evento que se repite tiene una por vez. */
export function occurrenceKey(
	occurrence: Pick<CalendarOccurrence, 'event_id' | 'occurrence_id'>
): string {
	return `${occurrence.event_id}#${occurrence.occurrence_id}`;
}

/**
 * La hora de una vez de evento como instante.
 *
 * El almacén manda todo en UTC. Un evento **flotante** —sin zona, «a las 10
 * donde esté»— viaja con su hora de pared escrita como si fuera UTC: las 10:00
 * flotantes llegan como `10:00Z`, y leídas como instante en Buenos Aires serían
 * las 7. Se arma la fecha local con esas mismas cifras.
 */
function instant(value: string, floating: boolean): Date | string {
	if (!value) return value;
	const date = new Date(value);
	if (Number.isNaN(date.getTime()) || !floating) return value;
	return new Date(
		date.getUTCFullYear(),
		date.getUTCMonth(),
		date.getUTCDate(),
		date.getUTCHours(),
		date.getUTCMinutes(),
		date.getUTCSeconds()
	);
}

/** Una vez de evento del almacén, como la dibujan el calendario y la lista. */
export function toCalendarEntry(occurrence: CalendarOccurrence, location?: string): CalendarEntry {
	const floating = occurrence.floating && !occurrence.all_day;
	return {
		id: occurrenceKey(occurrence),
		title: occurrence.title,
		start: instant(occurrence.start, floating),
		end: occurrence.end ? instant(occurrence.end, floating) : null,
		allDay: occurrence.all_day,
		location: location || null,
		calendar: occurrence.calendar ?? null,
		color: occurrence.color ?? null,
	};
}

/**
 * Lo que se le pide al almacén para dibujar un mes: las seis semanas de la
 * cuadrícula, de la medianoche local del primer día a la del día después del
 * último, en RFC 3339. Así los días de los meses vecinos que se ven también
 * llevan su punto.
 */
export function gridRange(month: IsoMonth, weekStart: number): { from: string; to: string } | null {
	const days = monthGrid(month, weekStart).flat();
	const first = days[0];
	const last = days.at(-1);
	if (!first || !last) return null;
	const from = parseIsoDate(first.date);
	const to = parseIsoDate(addDays(last.date, 1));
	if (!from || !to) return null;
	return { from: from.toISOString(), to: to.toISOString() };
}

/** «mañana», «hoy», «ayer» en el idioma, o nada si el día está más lejos. */
export function relativeDayLabel(date: IsoDate, today: IsoDate, locale?: string): string {
	const a = parseIsoDate(date);
	const b = parseIsoDate(today);
	if (!a || !b) return '';
	const diff = Math.round((a.getTime() - b.getTime()) / 86_400_000);
	if (Math.abs(diff) > 1) return '';
	return new Intl.RelativeTimeFormat(locale, { numeric: 'auto' }).format(diff, 'day');
}

// ── El clima ───────────────────────────────────────────────────────────────

const CODES = weatherCodes as CodeDataType;

/** El icono del tema para un código de condición de Open-Meteo, de día o de noche. */
export function weatherIcon(code: number | null | undefined, isDay: boolean): string {
	const info = code === null || code === undefined ? undefined : CODES[String(code)];
	if (!info) return 'weather-severe-alert';
	return isDay ? info.day.image : info.night.image;
}

/** Lo que el tablero lee de la respuesta de Open-Meteo. Todo puede faltar: lo guardado puede ser de una versión anterior del pedido. */
export interface ForecastData {
	current?: {
		time?: string;
		temperature_2m?: number;
		is_day?: number;
		weather_code?: number;
		relative_humidity_2m?: number;
		apparent_temperature?: number;
		wind_speed_10m?: number;
	};
	current_units?: Record<string, string>;
	hourly?: {
		time?: string[];
		temperature_2m?: number[];
		weather_code?: number[];
		is_day?: number[];
		precipitation_probability?: number[];
		relative_humidity_2m?: number[];
		apparent_temperature?: number[];
		wind_speed_10m?: number[];
	};
	hourly_units?: Record<string, string>;
	daily?: {
		time?: string[];
		weather_code?: number[];
		temperature_2m_max?: number[];
		temperature_2m_min?: number[];
		precipitation_probability_max?: number[];
		wind_speed_10m_max?: number[];
	};
}

/**
 * Una hora de Open-Meteo como instante local.
 *
 * Con `timezone=auto` las horas llegan sin zona («2026-03-22T22:00»), en la del
 * lugar, que es la de la sesión: el lugar se deduce de esa misma zona. Se arma
 * por partes porque `new Date("…T22:00")` sin zona lo lee bien en local, pero
 * `"2026-03-22"` lo leería en UTC.
 */
function localHour(value: string): Date | null {
	const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(value);
	if (!match) return null;
	return new Date(
		Number(match[1]),
		Number(match[2]) - 1,
		Number(match[3]),
		Number(match[4]),
		Number(match[5])
	);
}

/** En qué posición de `hourly.time` está la hora en curso, o −1. */
function hourIndexOf(weather: ForecastData | null | undefined, now: Date): number {
	const thisHour = new Date(now);
	thisHour.setMinutes(0, 0, 0);
	return (weather?.hourly?.time ?? []).findIndex(
		(time) => localHour(time)?.getTime() === thisHour.getTime()
	);
}

/** Cuántas horas van en el arco: la actual y las seis que siguen. Con nueve, en la columna de 330 px del centro se pisaban. */
export const ARC_HOURS = 7;

/**
 * Las horas del arco: la actual y las que siguen, con su icono y su
 * temperatura, y cuál es la actual.
 *
 * Una ventana fija y no «lo que queda del día»: a las 23 lo que queda del día
 * es una hora, y el arco quedaría vacío justo cuando se mira el pronóstico de
 * la noche. Si lo guardado no tiene horas —un pronóstico pedido antes de que el
 * escritorio las pidiera— o la hora actual ya no está, no hay arco.
 */
export function upcomingHours(
	weather: ForecastData | null | undefined,
	now: Date,
	describe: (code: number) => string = () => '',
	count = ARC_HOURS
): { hours: ForecastHour[]; current: number | null } {
	const hourly = weather?.hourly;
	const times = hourly?.time ?? [];
	const temperatures = hourly?.temperature_2m ?? [];
	const codes = hourly?.weather_code ?? [];
	if (times.length === 0 || temperatures.length === 0) return { hours: [], current: null };

	const start = hourIndexOf(weather, now);
	if (start < 0) return { hours: [], current: null };

	const hours: ForecastHour[] = [];
	for (let index = start; index < Math.min(times.length, start + count); index += 1) {
		const time = localHour(times[index] as string);
		const temperature = temperatures[index];
		if (!time || typeof temperature !== 'number') continue;
		const code = codes[index];
		const isDay = (hourly?.is_day?.[index] ?? 1) === 1;
		hours.push({
			time,
			icon: weatherIcon(code, isDay),
			temperature,
			description: typeof code === 'number' ? describe(code) : '',
		});
	}
	return { hours, current: hours.length > 0 ? 0 : null };
}

/** El clima de un día, como lo dibuja la columna de la derecha. */
export interface DayWeather {
	date: IsoDate;
	/** La de ahora para hoy; la máxima para otro día. */
	temperature: number;
	max: number | null;
	min: number | null;
	code: number | null;
	isDay: boolean;
	wind: number | null;
	humidity: number | null;
	rain: number | null;
	feelsLike: number | null;
	units: { temperature: string; wind: string };
}

function numbers(values: Array<number | null | undefined>): number[] {
	return values.filter(
		(value): value is number => typeof value === 'number' && Number.isFinite(value)
	);
}

/** Los valores por hora de un día, de un arreglo paralelo a `hourly.time`. */
function hoursOf(
	weather: ForecastData,
	date: IsoDate,
	key: keyof NonNullable<ForecastData['hourly']>
): number[] {
	const times = weather.hourly?.time ?? [];
	const values = (weather.hourly?.[key] ?? []) as Array<number | null | undefined>;
	return numbers(times.map((time, index) => (time.startsWith(date) ? values[index] : null)));
}

const mean = (values: number[]) =>
	values.length ? values.reduce((a, b) => a + b, 0) / values.length : null;
const maxOf = (values: number[]) => (values.length ? Math.max(...values) : null);

/**
 * El clima de un día del pronóstico, o `null` si el día no está en él.
 *
 * **Hoy** es lo de ahora: la temperatura, la condición, el viento, la humedad y
 * la sensación de `current`, y la lluvia de la hora actual. **Otro día** es su
 * resumen: la máxima, la condición del día, el viento máximo, la humedad media
 * de sus horas, la probabilidad máxima de lluvia y la sensación máxima.
 */
export function dayWeather(
	weather: ForecastData | null | undefined,
	date: IsoDate,
	now: Date
): DayWeather | null {
	const daily = weather?.daily;
	const index = daily?.time?.indexOf(date) ?? -1;
	if (!weather || !daily || index < 0) return null;

	const max = daily.temperature_2m_max?.[index] ?? null;
	const min = daily.temperature_2m_min?.[index] ?? null;
	const units = {
		temperature:
			weather.current_units?.temperature_2m ?? weather.hourly_units?.temperature_2m ?? '°C',
		wind: weather.current_units?.wind_speed_10m ?? weather.hourly_units?.wind_speed_10m ?? 'km/h',
	};
	const current = weather.current;

	if (date === toIsoDate(now) && typeof current?.temperature_2m === 'number') {
		const hourIndex = hourIndexOf(weather, now);
		return {
			date,
			temperature: current.temperature_2m,
			max,
			min,
			code: current.weather_code ?? daily.weather_code?.[index] ?? null,
			isDay: (current.is_day ?? 1) === 1,
			wind: current.wind_speed_10m ?? null,
			humidity: current.relative_humidity_2m ?? null,
			rain:
				(hourIndex >= 0 ? weather.hourly?.precipitation_probability?.[hourIndex] : undefined) ??
				daily.precipitation_probability_max?.[index] ??
				null,
			feelsLike: current.apparent_temperature ?? null,
			units,
		};
	}

	if (typeof max !== 'number') return null;
	return {
		date,
		temperature: max,
		max,
		min,
		code: daily.weather_code?.[index] ?? null,
		isDay: true,
		wind: daily.wind_speed_10m_max?.[index] ?? maxOf(hoursOf(weather, date, 'wind_speed_10m')),
		humidity: mean(hoursOf(weather, date, 'relative_humidity_2m')),
		rain:
			daily.precipitation_probability_max?.[index] ??
			maxOf(hoursOf(weather, date, 'precipitation_probability')),
		feelsLike: maxOf(hoursOf(weather, date, 'apparent_temperature')),
		units,
	};
}

/** Cuántos km/h es una unidad de viento de Open-Meteo. */
const TO_KMH: Record<string, number> = { 'm/s': 3.6, 'mp/h': 1.609, mph: 1.609, kn: 1.852 };

/**
 * Qué tan lleno va cada anillo, de 0 a 100.
 *
 * La humedad y la lluvia ya son porcentajes. El viento va contra 60 km/h —un
 * temporal fuerte llena el anillo— y la sensación, de −10° a 40°: un anillo de
 * temperatura que se llena con calor y se vacía con frío se lee sin número.
 * En m/s o °F se convierte antes, para que el mismo viento llene lo mismo.
 */
export function ringFill(
	kind: 'wind' | 'humidity' | 'rain' | 'feelsLike',
	value: number | null,
	unit = ''
): number | null {
	if (value === null || !Number.isFinite(value)) return null;
	const clamp = (n: number) => Math.min(100, Math.max(0, n));
	switch (kind) {
		case 'humidity':
		case 'rain':
			return clamp(value);
		case 'wind': {
			const kmh = value * (TO_KMH[unit] ?? 1);
			return clamp((kmh / 60) * 100);
		}
		case 'feelsLike': {
			const celsius = unit.includes('F') ? ((value - 32) * 5) / 9 : value;
			return clamp(((celsius + 10) / 50) * 100);
		}
	}
}
