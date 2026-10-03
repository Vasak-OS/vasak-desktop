/**
 * El clima compartido de todo el escritorio: lo usan el widget, el panel, el
 * menú y el tablero de fecha (vasak-desktop#130).
 *
 * Se prueba el módulo de verdad, con un `__TAURI_INTERNALS__` de mentira —el
 * cache de Rust contesta lo que la prueba decide— y un `fetch` de mentira. El
 * estado es del módulo y arranca una sola vez, así que las pruebas van en
 * orden: primero lo guardado, después un pedido que sale bien y uno que falla.
 */
import { afterAll, beforeAll, describe, expect, test } from 'bun:test';
import { forecastUrl } from '@/tools/forecast-url';

type Handler = (args: Record<string, unknown>) => unknown;

const calls: Array<{ cmd: string; args: Record<string, unknown> }> = [];
const handlers: Record<string, Handler> = {};
const requested: string[] = [];
let respond: (url: string) => Promise<Response> = async () => new Response('{}');

const saved = {
	current: { temperature_2m: 10.6, is_day: 0, weather_code: 0 },
	daily: {
		time: ['2026-03-22', '2026-03-23', '2026-03-24'],
		temperature_2m_max: [12, 15, 14],
		temperature_2m_min: [3, 8, 6],
		weather_code: [0, 61, 3],
	},
};

const realFetch = globalThis.fetch;
let useWeather: typeof import('@/tools/composables/useWeather').useWeather;
let weather: ReturnType<typeof useWeather>;

/** Espera a que se terminen de encadenar las promesas del módulo. */
const settle = () => new Promise((done) => setTimeout(done, 20));

/**
 * El `window` que había antes, para devolverlo al terminar: con el DOM del
 * `preload` (`tests/support/dom.ts`) es el de happy-dom, y borrarlo deja sin
 * `window` a las pruebas que montan componentes después.
 */
const previousWindow = (globalThis as any).window;

beforeAll(async () => {
	(globalThis as any).window = {
		__TAURI_INTERNALS__: {
			transformCallback: () => 1,
			invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
				calls.push({ cmd, args });
				return handlers[cmd]?.(args) ?? null;
			},
		},
	};
	globalThis.fetch = (async (input: string) => {
		requested.push(String(input));
		return respond(String(input));
	}) as typeof fetch;
	handlers.weather_cached = () => ({ datos: saved, edad_segundos: 10, vencido: false });
	handlers.weather_claim = () => false;

	({ useWeather } = await import('@/tools/composables/useWeather'));
	weather = useWeather();
	await settle();
});

afterAll(() => {
	(globalThis as any).window = previousWindow;
	globalThis.fetch = realFetch;
});

describe('el clima compartido', () => {
	test('arranca con lo guardado del lado de Rust y, si no le toca, no pide nada', () => {
		expect(calls.map((call) => call.cmd)).toEqual(
			expect.arrayContaining(['weather_cached', 'weather_claim'])
		);
		expect(requested).toEqual([]);
		expect(weather.current.value?.temperature_2m).toBe(10.6);
		expect(weather.dayOrNight.value).toBe('night');
		expect(weather.failed.value).toBe(false);
		expect(weather.loading.value).toBe(false);
	});

	test('el pronóstico de mañana en adelante viene aplanado', () => {
		expect(weather.upcoming.value).toEqual([
			{ date: '2026-03-23', min: 8, max: 15, code: 61 },
			{ date: '2026-03-24', min: 6, max: 14, code: 3 },
		]);
	});

	test('si le toca, pide el pronóstico con el pedido compartido y lo guarda para las demás ventanas', async () => {
		const fresh = { ...saved, current: { temperature_2m: 18, is_day: 1, weather_code: 2 } };
		handlers.weather_claim = () => true;
		handlers.weather_place = () => ({ lat: -34.6, lon: -58.38 });
		respond = async () => new Response(JSON.stringify(fresh));

		await weather.refresh();

		expect(requested).toEqual([forecastUrl({ lat: -34.6, lon: -58.38 })]);
		expect(weather.current.value?.temperature_2m).toBe(18);
		expect(weather.dayOrNight.value).toBe('day');
		const store = calls.find((call) => call.cmd === 'weather_store');
		expect(store?.args).toEqual({ datos: fresh, lugar: { lat: -34.6, lon: -58.38 } });
	});

	test('sin coordenadas guardadas, las deduce de la zona horaria con el mismo proveedor', async () => {
		handlers.weather_place = () => null;
		requested.length = 0;
		respond = async (url) =>
			new URL(url).hostname === 'geocoding-api.open-meteo.com'
				? new Response(JSON.stringify({ results: [{ latitude: 1, longitude: 2 }] }))
				: new Response(JSON.stringify(saved));

		await weather.refresh();

		expect(new URL(requested[0] as string).origin).toBe('https://geocoding-api.open-meteo.com');
		expect(requested[1]).toBe(forecastUrl({ lat: 1, lon: 2 }));
	});

	test('si el pedido falla devuelve el turno, y con algo ya mostrado no dice que falló', async () => {
		handlers.weather_place = () => ({ lat: 1, lon: 2 });
		respond = async () => new Response('no', { status: 503 });
		const before = calls.length;

		await weather.refresh();

		expect(calls.slice(before).map((call) => call.cmd)).toContain('weather_release');
		expect(weather.failed.value).toBe(false);
		expect(weather.current.value).not.toBeNull();
	});

	test('sin puente con Rust no pide nada: sería un pedido por ventana', async () => {
		handlers.weather_claim = () => {
			throw new Error('sin puente');
		};
		requested.length = 0;

		await weather.refresh();

		expect(requested).toEqual([]);
	});
});
