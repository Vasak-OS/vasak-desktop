/**
 * Lo que dice la píldora del reloj (vasak-desktop#151, rediseño #168).
 *
 * `clockParts` es una función pura: se prueba sin montar nada. El rediseño pide
 * el día de la semana y el «número + mes» por separado, para apilarlos en dos
 * renglones al lado de la hora alta.
 */
import { describe, expect, test } from 'bun:test';
import { clockParts } from '../src/tools/panel-clock';

describe('las partes del reloj', () => {
	test('la hora va con dos dígitos', () => {
		const parts = clockParts(new Date(2026, 2, 22, 9, 5), 'es');
		expect(parts.time).toBe('09:05');
		expect(parts.hour).toBe('09');
		expect(parts.minute).toBe('05');
	});

	test('el día de la semana y el número + mes van separados', () => {
		// El renglón de arriba es sólo el día; el de abajo, el número y el mes.
		const parts = clockParts(new Date(2026, 2, 22, 9, 5), 'es');
		expect(parts.dayMonth).toBe('22 de marzo');
		expect(parts.weekday).not.toBe(parts.dayMonth);
		expect(parts.weekday).not.toContain('22');
	});

	test('las dos partes son los pedazos de la fecha entera', () => {
		// Juntas reconstruyen la fecha corta: el día abre, el número y el mes
		// la cierran. Así el globo y los renglones dicen lo mismo.
		const parts = clockParts(new Date(2026, 2, 22, 9, 5), 'es');
		expect(parts.date.startsWith(parts.weekday)).toBe(true);
		expect(parts.date.includes(parts.dayMonth)).toBe(true);
	});

	test('el globo lleva el año; los renglones, no', () => {
		const parts = clockParts(new Date(2026, 2, 22, 9, 5), 'es');
		expect(parts.longDate).toContain('2026');
		expect(parts.dayMonth).not.toContain('2026');
		expect(parts.weekday).not.toContain('2026');
	});

	test('un idioma que Intl no entiende no tira abajo el reloj', () => {
		const parts = clockParts(new Date(2026, 2, 22, 9, 5), 'no-es-un-idioma');
		expect(parts.time).toBe('09:05');
		expect(parts.weekday.length).toBeGreaterThan(0);
		expect(parts.dayMonth.length).toBeGreaterThan(0);
	});
});
