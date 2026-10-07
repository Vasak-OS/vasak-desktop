/**
 * Lo que el detalle de la luz nocturna necesita sin Vue (vasak-desktop#178):
 * los límites del deslizador y las coordenadas como texto.
 */

/** Los límites de `wlsunset` que acepta el plugin. */
export const MIN_TEMPERATURE = 1000;
export const TEMPERATURE_STEP = 100;

/**
 * La temperatura de noche más alta que se puede elegir: un paso por debajo de
 * la de día. El plugin rechaza una noche igual o más fría que el día, así que
 * el deslizador no deja llegar ahí.
 */
export function maxNightTemperature(dayTemperature: number): number {
	return Math.max(MIN_TEMPERATURE, dayTemperature - TEMPERATURE_STEP);
}

/** Una coordenada para el campo de texto: vacío si no hay. */
export function formatCoordinate(value: number | null): string {
	return value === null || !Number.isFinite(value) ? '' : String(value);
}

/**
 * El texto del campo como coordenada: `null` si quedó vacío, `undefined` si
 * no es un número dentro de `limit` (90 para la latitud, 180 para la
 * longitud). Acepta la coma decimal, que es la que se escribe en castellano.
 */
export function parseCoordinate(text: string, limit: number): number | null | undefined {
	const trimmed = text.trim().replace(',', '.');
	if (trimmed === '') return null;
	if (!/^[-+]?\d+(\.\d+)?$/.test(trimmed)) return undefined;
	const value = Number(trimmed);
	return Math.abs(value) <= limit ? value : undefined;
}
