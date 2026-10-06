/**
 * Las cuentas del tablero de fecha: de dónde sale cada cosa que dibuja.
 *
 * Los componentes (el mes, la lista, el arco, los anillos) los prueba
 * vue-libvasak montados. Acá se prueba cómo el escritorio les arma los datos a
 * partir de lo que ya tiene: el pronóstico de Open-Meteo que comparte con el
 * widget y el menú, y las veces de eventos del almacén de vasak-accounts.
 *
 * La zona horaria se fija en la de Buenos Aires (UTC−3), donde un evento
 * flotante o de día completo leído como instante cae corrido.
 */

process.env.TZ = 'America/Argentina/Buenos_Aires';

import { describe, expect, test } from 'bun:test';
import { entriesOn, markedDates } from '@vasakgroup/vue-libvasak';
import type { CalendarOccurrence } from '@/services/calendar.service';
import {
	ARC_HOURS,
	type DayWeather,
	dayWeather,
	type ForecastData,
	gridRange,
	occurrenceKey,
	relativeDayLabel,
	ringFill,
	toCalendarEntry,
	upcomingHours,
	weatherIcon,
	weatherRings,
} from '@/tools/date-board';

const occurrence = (extra: Partial<CalendarOccurrence> = {}): CalendarOccurrence => ({
	event_id: 'cuenta/7',
	occurrence_id: '',
	calendar_id: 'cuenta/1',
	title: 'Daily',
	start: '2026-03-23T13:00:00Z',
	end: '2026-03-23T13:30:00Z',
	all_day: false,
	floating: false,
	color: '#1e88e5',
	calendar: 'Trabajo',
	...extra,
});

describe('los eventos del almacén', () => {
	test('una vez de evento pasa a la forma del calendario, con su lugar y su calendario', () => {
		const entry = toCalendarEntry(occurrence(), 'Sala 3');

		expect(entry).toEqual({
			id: 'cuenta/7#',
			title: 'Daily',
			start: '2026-03-23T13:00:00Z',
			end: '2026-03-23T13:30:00Z',
			allDay: false,
			location: 'Sala 3',
			calendar: 'Trabajo',
			color: '#1e88e5',
		});
		expect(toCalendarEntry(occurrence({ calendar: null }), '').location).toBeNull();
	});

	test('cada vez de un evento que se repite tiene su clave', () => {
		expect(occurrenceKey(occurrence({ occurrence_id: '5' }))).not.toBe(
			occurrenceKey(occurrence({ occurrence_id: '6' }))
		);
	});

	test('un evento flotante se lee con su hora de pared, no corrido tres horas', () => {
		// «A las 10 donde esté»: el almacén lo manda como 10:00Z.
		const entry = toCalendarEntry(
			occurrence({ start: '2026-03-23T10:00:00Z', end: '2026-03-23T11:00:00Z', floating: true })
		);

		expect((entry.start as Date).getHours()).toBe(10);
		expect((entry.end as Date).getHours()).toBe(11);
	});

	test('uno de día completo cae en su día, no en el anterior', () => {
		const entry = toCalendarEntry(
			occurrence({
				start: '2026-03-23T00:00:00Z',
				end: '2026-03-24T00:00:00Z',
				all_day: true,
				floating: true,
			})
		);

		expect(markedDates([entry])).toEqual(['2026-03-23']);
		expect(entriesOn([entry], '2026-03-22')).toEqual([]);
	});

	test('se pide la cuadrícula entera del mes, de medianoche a medianoche local', () => {
		// Marzo de 2026 con la semana en lunes: del 23 de febrero al 5 de abril.
		const range = gridRange('2026-03', 1);

		expect(range?.from).toBe(new Date(2026, 1, 23).toISOString());
		expect(range?.to).toBe(new Date(2026, 3, 6).toISOString());
		expect(gridRange('no-es-un-mes', 1)).toBeNull();
	});

	test('mañana, hoy y ayer se dicen; lo demás no', () => {
		expect(relativeDayLabel('2026-03-23', '2026-03-22', 'es')).toBe('mañana');
		expect(relativeDayLabel('2026-03-22', '2026-03-22', 'es')).toBe('hoy');
		expect(relativeDayLabel('2026-03-21', '2026-03-22', 'es')).toBe('ayer');
		expect(relativeDayLabel('2026-03-25', '2026-03-22', 'es')).toBe('');
	});
});

/** Un pronóstico como el de Open-Meteo con `timezone=auto`, desde el 22/03 a las 00. */
function forecast(): ForecastData {
	const time: string[] = [];
	const temperature_2m: number[] = [];
	const weather_code: number[] = [];
	const is_day: number[] = [];
	const precipitation_probability: number[] = [];
	const relative_humidity_2m: number[] = [];
	const apparent_temperature: number[] = [];
	const wind_speed_10m: number[] = [];
	for (let h = 0; h < 48; h += 1) {
		const day = 22 + Math.floor(h / 24);
		const hour = h % 24;
		time.push(`2026-03-${day}T${String(hour).padStart(2, '0')}:00`);
		temperature_2m.push(h);
		weather_code.push(h < 24 ? 0 : 61);
		is_day.push(hour >= 7 && hour < 20 ? 1 : 0);
		precipitation_probability.push(h < 24 ? 10 : 80);
		relative_humidity_2m.push(h < 24 ? 50 : 90);
		apparent_temperature.push(h - 2);
		wind_speed_10m.push(h < 24 ? 10 : 30);
	}
	return {
		current: {
			temperature_2m: 10.6,
			is_day: 0,
			weather_code: 0,
			relative_humidity_2m: 71,
			apparent_temperature: 9.1,
			wind_speed_10m: 6,
		},
		current_units: { temperature_2m: '°C', wind_speed_10m: 'km/h' },
		hourly: {
			time,
			temperature_2m,
			weather_code,
			is_day,
			precipitation_probability,
			relative_humidity_2m,
			apparent_temperature,
			wind_speed_10m,
		},
		daily: {
			time: ['2026-03-22', '2026-03-23'],
			weather_code: [0, 61],
			temperature_2m_max: [12, 15],
			temperature_2m_min: [3, 8],
			precipitation_probability_max: [10, 80],
			wind_speed_10m_max: [14, 35],
		},
	};
}

describe('el arco de las próximas horas', () => {
	test('empieza en la hora en curso, que es la resaltada, y sigue seis más', () => {
		const { hours, current } = upcomingHours(forecast(), new Date(2026, 2, 22, 22, 41), (code) =>
			code === 0 ? 'Despejado' : 'Lluvia'
		);

		expect(hours).toHaveLength(ARC_HOURS);
		expect(ARC_HOURS).toBe(7);
		expect(current).toBe(0);
		expect(new Date(hours[0]?.time ?? 0).getHours()).toBe(22);
		expect(hours[0]?.temperature).toBe(22);
		expect(hours[0]?.description).toBe('Despejado');
		// De noche, el icono de noche; y pasada la medianoche, el día siguiente.
		expect(hours[0]?.icon).toBe('weather-clear-night');
		expect(new Date(hours[2]?.time ?? 0).getDate()).toBe(23);
		expect(hours[2]?.description).toBe('Lluvia');
	});

	test('cerca del final del pronóstico trae las que queden', () => {
		expect(upcomingHours(forecast(), new Date(2026, 2, 23, 21, 5)).hours).toHaveLength(3);
	});

	test('sin horas guardadas, o sin la hora en curso, no hay arco', () => {
		const old: ForecastData = {
			current: { temperature_2m: 10 },
			daily: { time: ['2026-03-22'], temperature_2m_max: [12] },
		};

		expect(upcomingHours(old, new Date(2026, 2, 22, 22))).toEqual({ hours: [], current: null });
		expect(upcomingHours(forecast(), new Date(2026, 2, 25, 10))).toEqual({
			hours: [],
			current: null,
		});
		expect(upcomingHours(null, new Date())).toEqual({ hours: [], current: null });
	});

	test('el icono sale del tema, con un respaldo estándar para un código que no se conoce', () => {
		expect(weatherIcon(0, true)).toBe('weather-clear');
		expect(weatherIcon(0, false)).toBe('weather-clear-night');
		expect(weatherIcon(1234, true)).toBe('weather-severe-alert');
		expect(weatherIcon(null, true)).toBe('weather-severe-alert');
	});
});

describe('el clima de un día', () => {
	test('hoy es lo de ahora: la temperatura, el viento, la humedad y la sensación de current', () => {
		const day = dayWeather(forecast(), '2026-03-22', new Date(2026, 2, 22, 22, 41));

		expect(day).toMatchObject({
			temperature: 10.6,
			max: 12,
			min: 3,
			code: 0,
			isDay: false,
			wind: 6,
			humidity: 71,
			feelsLike: 9.1,
			units: { temperature: '°C', wind: 'km/h' },
		});
		// La lluvia, de la hora en curso.
		expect(day?.rain).toBe(10);
	});

	test('otro día es su resumen: la máxima, el viento máximo, la humedad media y la lluvia más probable', () => {
		const day = dayWeather(forecast(), '2026-03-23', new Date(2026, 2, 22, 22, 41));

		expect(day).toMatchObject({
			temperature: 15,
			max: 15,
			min: 8,
			code: 61,
			wind: 35,
			humidity: 90,
			rain: 80,
		});
		// La sensación máxima de sus horas: la última, 47 − 2.
		expect(day?.feelsLike).toBe(45);
	});

	test('un día fuera del pronóstico no tiene clima', () => {
		expect(dayWeather(forecast(), '2026-04-10', new Date(2026, 2, 22))).toBeNull();
		expect(dayWeather(null, '2026-03-22', new Date(2026, 2, 22))).toBeNull();
	});

	test('lo guardado por una versión anterior, sin horas ni viento, igual da el día', () => {
		const old: ForecastData = {
			current: { temperature_2m: 10, is_day: 1, weather_code: 3 },
			current_units: { temperature_2m: '°C' },
			daily: {
				time: ['2026-03-22'],
				weather_code: [3],
				temperature_2m_max: [12],
				temperature_2m_min: [3],
			},
		};
		const day = dayWeather(old, '2026-03-22', new Date(2026, 2, 22, 12));

		expect(day).toMatchObject({
			temperature: 10,
			code: 3,
			wind: null,
			humidity: null,
			rain: null,
			feelsLike: null,
		});
	});
});

describe('qué tan lleno va cada anillo', () => {
	test('humedad y lluvia son porcentajes; sin dato, sin anillo', () => {
		expect(ringFill('humidity', 71)).toBe(71);
		expect(ringFill('rain', 140)).toBe(100);
		expect(ringFill('rain', null)).toBeNull();
	});

	test('el viento va contra 60 km/h, y el mismo viento llena lo mismo en otra unidad', () => {
		expect(ringFill('wind', 30, 'km/h')).toBe(50);
		expect(ringFill('wind', 30 / 3.6, 'm/s')).toBeCloseTo(50, 5);
		expect(ringFill('wind', 200, 'km/h')).toBe(100);
	});

	test('la sensación, de −10° a 40°, también en Fahrenheit', () => {
		expect(ringFill('feelsLike', 15, '°C')).toBe(50);
		expect(ringFill('feelsLike', 59, '°F')).toBeCloseTo(50, 5);
		expect(ringFill('feelsLike', -30, '°C')).toBe(0);
	});
});

describe('los cuatro anillos del día', () => {
	const labels = { wind: 'Viento', humidity: 'Humedad', rain: 'Lluvia', feelsLike: 'Sensación' };
	const day: DayWeather = {
		date: '2026-03-22',
		temperature: 20,
		max: 24,
		min: 14,
		code: 2,
		isDay: true,
		wind: 30,
		humidity: 54,
		rain: 10,
		feelsLike: 15,
		units: { temperature: '°C', wind: 'km/h' },
	};

	test('sin día, ninguno', () => {
		expect(weatherRings(null, labels)).toEqual([]);
	});

	test('viento con su unidad, humedad y lluvia en %, sensación en grados', () => {
		const rings = weatherRings(day, labels);
		expect(rings.map((r) => r.key)).toEqual(['wind', 'humidity', 'rain', 'feelsLike']);
		expect(rings[0]).toMatchObject({ label: 'Viento', display: '30', unit: 'km/h', value: 50 });
		expect(rings[1]).toMatchObject({ display: '54%', value: 54 });
		expect(rings[2]).toMatchObject({ display: '10%', value: 10 });
		expect(rings[3]).toMatchObject({ display: '15°', value: 50 });
	});

	test('un dato que falta sale como «–» y sin anillo', () => {
		const rings = weatherRings({ ...day, wind: null, humidity: null }, labels);
		expect(rings[0]).toMatchObject({ display: '–', value: null });
		expect(rings[1]).toMatchObject({ display: '–', value: null });
	});
});
