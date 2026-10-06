/**
 * Las cuentas del widget de clima (vasak-desktop#167).
 *
 * Qué día se muestra, qué entra según el tamaño de la celda y cómo se arman las
 * tarjetas pasantes viven en `tools/weather-widget.ts` para poder probarse sin
 * montar el componente.
 */
import { describe, expect, test } from 'bun:test';
import type { IsoDate } from '@vasakgroup/vue-libvasak';
import {
	canStep,
	dayIndex,
	dayOptions,
	stepDate,
	weatherLayout,
} from '../src/tools/weather-widget';

const DAYS = [
	'2026-10-06',
	'2026-10-07',
	'2026-10-08',
	'2026-10-09',
] as unknown as IsoDate[];

describe('ubicar y moverse entre días', () => {
	test('dayIndex encuentra la fecha, o −1', () => {
		expect(dayIndex(DAYS, DAYS[1])).toBe(1);
		expect(dayIndex(DAYS, '2020-01-01' as IsoDate)).toBe(-1);
	});

	test('stepDate avanza y retrocede, y se queda en los bordes', () => {
		expect(stepDate(DAYS, DAYS[1], 1)).toBe(DAYS[2]);
		expect(stepDate(DAYS, DAYS[1], -1)).toBe(DAYS[0]);
		// En el primero, un paso atrás se queda en el primero.
		expect(stepDate(DAYS, DAYS[0], -1)).toBe(DAYS[0]);
		// En el último, un paso adelante se queda en el último.
		expect(stepDate(DAYS, DAYS[3], 1)).toBe(DAYS[3]);
		// Una fecha que no está no da nada.
		expect(stepDate(DAYS, '2020-01-01' as IsoDate, 1)).toBeNull();
	});

	test('canStep dice si el paso cambia de día', () => {
		expect(canStep(DAYS, DAYS[0], -1)).toBe(false);
		expect(canStep(DAYS, DAYS[0], 1)).toBe(true);
		expect(canStep(DAYS, DAYS[3], 1)).toBe(false);
		expect(canStep(DAYS, DAYS[3], -1)).toBe(true);
		expect(canStep(DAYS, '2020-01-01' as IsoDate, 1)).toBe(false);
	});
});

describe('las tarjetas pasantes', () => {
	test('un día por opción, con el nombre corto y la máxima', () => {
		const options = dayOptions(
			DAYS,
			[24, 20, null, 25],
			(date) => `d-${date.slice(-2)}`,
			(value) => (value === null ? '–' : `${value}°`)
		);
		expect(options).toHaveLength(4);
		expect(options[0]).toEqual({ value: DAYS[0], label: 'd-06', badge: '24°' });
		// La máxima que falta sale como «–», no rompe.
		expect(options[2].badge).toBe('–');
	});
});

describe('qué entra según el tamaño de la celda', () => {
	test('baja y ancha (la fila de hoy): resumen en fila, sin anillos ni tarjetas', () => {
		const l = weatherLayout(240, 120, true, 7);
		expect(l.compactRow).toBe(true);
		expect(l.showRings).toBe(false);
		expect(l.showStrip).toBe(false);
	});

	test('con alto aparecen los anillos; en ancho van de a cuatro', () => {
		expect(weatherLayout(384, 300, false, 7).showRings).toBe(true);
		expect(weatherLayout(384, 300, false, 7).ringColumns).toBe(4);
		expect(weatherLayout(200, 300, false, 7).ringColumns).toBe(2);
	});

	test('las tarjetas pasantes, sólo con mucho alto, varias fechas y no en hoy', () => {
		expect(weatherLayout(400, 420, false, 7).showStrip).toBe(true);
		expect(weatherLayout(400, 420, true, 7).showStrip).toBe(false);
		expect(weatherLayout(400, 420, false, 1).showStrip).toBe(false);
		expect(weatherLayout(400, 380, false, 7).showStrip).toBe(false);
	});
});
