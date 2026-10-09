/**
 * Las cuentas del widget de clima, sin Vue (vasak-desktop#167).
 *
 * Lo que decide qué día se muestra, qué entra según el tamaño de la celda y cómo
 * se arman las tarjetas pasantes vive acá para poder probarse; el componente sólo
 * las dibuja. El clima de cada día y los anillos salen de `date-board.ts`, que lo
 * comparte con el tablero de fecha.
 */
import type { IsoDate, SegmentedOption } from '@vasakgroup/vue-libvasak';

/** Dónde está una fecha en la lista del pronóstico, o −1. */
export function dayIndex(days: readonly IsoDate[], date: IsoDate): number {
	return days.indexOf(date);
}

/**
 * La fecha a `amount` días de la actual, acotada a la lista; `null` si la actual
 * no está. En los bordes se queda en el primero o el último, no se sale.
 */
export function stepDate(
	days: readonly IsoDate[],
	current: IsoDate,
	amount: number
): IsoDate | null {
	const index = days.indexOf(current);
	if (index < 0) return null;
	const next = days[Math.min(days.length - 1, Math.max(0, index + amount))];
	return next ?? null;
}

/** Si se puede dar un paso de `amount` días sin salirse de la lista. */
export function canStep(days: readonly IsoDate[], current: IsoDate, amount: number): boolean {
	const index = days.indexOf(current);
	if (index < 0) return false;
	const target = index + amount;
	return target >= 0 && target < days.length && target !== index;
}

/**
 * Los días como opciones del control segmentado: el nombre corto y la máxima.
 *
 * Los textos los arma quien llama —dependen del idioma y de la unidad—, así que
 * se pasan las dos funciones de formato.
 */
export function dayOptions(
	days: readonly IsoDate[],
	maxTemps: readonly (number | null | undefined)[],
	shortWeekday: (date: IsoDate) => string,
	formatTemp: (value: number | null) => string
): SegmentedOption<IsoDate>[] {
	return days.map((date, index) => ({
		value: date,
		label: shortWeekday(date),
		badge: formatTemp(maxTemps[index] ?? null),
	}));
}

/** Qué partes del widget entran, según el tamaño medido de la celda. */
export interface WeatherLayout {
	/** Los cuatro anillos entran cuando la celda tiene alto. */
	showRings: boolean;
	/** Las tarjetas pasantes, con más alto todavía y más de un día. */
	showStrip: boolean;
	/** En celda angosta los anillos van de a dos; con ancho, los cuatro en fila. */
	ringColumns: number;
	/** En celda muy baja —la variante de hoy— el resumen va en fila. */
	compactRow: boolean;
}

export function weatherLayout(
	width: number,
	height: number,
	soloHoy: boolean,
	dayCount: number
): WeatherLayout {
	return {
		showRings: height >= 260,
		showStrip: !soloHoy && height >= 400 && dayCount > 1,
		ringColumns: width >= 260 ? 4 : 2,
		compactRow: height < 150,
	};
}
