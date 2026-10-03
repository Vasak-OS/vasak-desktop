import type { WeatherPlace } from '@/services/weather.service';

/**
 * Lo que se le pide a Open-Meteo, en un solo pedido.
 *
 * `current` y `daily` son lo de siempre —el widget, el panel y el menú—, más lo
 * que necesita el tablero de fecha (vasak-desktop#130): el viento, la humedad y
 * la sensación térmica de ahora, y **por hora** la temperatura, la condición, la
 * probabilidad de lluvia, la humedad, la sensación y el viento, de donde salen
 * el arco de las próximas horas y los anillos de cualquier día del pronóstico.
 * Todo es del mismo servicio y del mismo pedido: no hay un tercero nuevo, y la
 * política de contenido ya permite `api.open-meteo.com`.
 */
export const FORECAST_FIELDS = {
	current: [
		'temperature_2m',
		'is_day',
		'weather_code',
		'relative_humidity_2m',
		'apparent_temperature',
		'wind_speed_10m',
	],
	hourly: [
		'temperature_2m',
		'weather_code',
		'is_day',
		'precipitation_probability',
		'relative_humidity_2m',
		'apparent_temperature',
		'wind_speed_10m',
	],
	daily: [
		'weather_code',
		'temperature_2m_max',
		'temperature_2m_min',
		'precipitation_probability_max',
		'wind_speed_10m_max',
	],
} as const;

/** La dirección del pronóstico para un lugar. */
export function forecastUrl(place: WeatherPlace): string {
	const query = [
		`latitude=${place.lat}`,
		`longitude=${place.lon}`,
		`current=${FORECAST_FIELDS.current.join(',')}`,
		`hourly=${FORECAST_FIELDS.hourly.join(',')}`,
		`daily=${FORECAST_FIELDS.daily.join(',')}`,
		'timezone=auto',
	];
	return `https://api.open-meteo.com/v1/forecast?${query.join('&')}`;
}
