import { describe, expect, test } from 'bun:test';
import { FORECAST_FIELDS, forecastUrl } from '@/tools/forecast-url';

/**
 * El pedido del clima, compartido por el widget, el panel, el menú y el
 * tablero de fecha (vasak-desktop#130).
 *
 * Es un solo pedido al mismo servicio: lo que suma el tablero —las horas, el
 * viento, la humedad, la sensación— va en la misma dirección, sin un tercero
 * nuevo que la política de contenido tendría que permitir.
 */
describe('el pedido del pronóstico', () => {
	const url = new URL(forecastUrl({ lat: -34.6, lon: -58.38 }));

	test('va a api.open-meteo.com, con el lugar y la zona del lugar', () => {
		expect(url.origin).toBe('https://api.open-meteo.com');
		expect(url.searchParams.get('latitude')).toBe('-34.6');
		expect(url.searchParams.get('longitude')).toBe('-58.38');
		expect(url.searchParams.get('timezone')).toBe('auto');
	});

	test('sigue trayendo lo que ya usaban el widget, el panel y el menú', () => {
		expect(url.searchParams.get('current')?.split(',')).toEqual(
			expect.arrayContaining(['temperature_2m', 'is_day', 'weather_code'])
		);
		expect(url.searchParams.get('daily')?.split(',')).toEqual(
			expect.arrayContaining(['weather_code', 'temperature_2m_max', 'temperature_2m_min'])
		);
	});

	test('y suma lo del tablero: las horas y los datos de los anillos', () => {
		expect(url.searchParams.get('hourly')?.split(',')).toEqual([...FORECAST_FIELDS.hourly]);
		expect(url.searchParams.get('current')?.split(',')).toEqual(
			expect.arrayContaining(['relative_humidity_2m', 'apparent_temperature', 'wind_speed_10m'])
		);
		expect(FORECAST_FIELDS.hourly).toContain('precipitation_probability');
	});
});
