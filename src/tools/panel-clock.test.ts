import { describe, expect, test } from 'bun:test';
import { clockParts } from './panel-clock';

describe('la píldora del reloj', () => {
	const sunday = new Date(2026, 2, 22, 9, 5, 0);

	test('la hora va con dos dígitos', () => {
		const parts = clockParts(sunday, 'es');
		expect(parts.time).toBe('09:05');
		expect([parts.hour, parts.minute]).toEqual(['09', '05']);
	});

	test('la fecha de abajo es la del día, en el idioma de la sesión', () => {
		expect(clockParts(sunday, 'es').date).toBe('domingo, 22 de marzo');
		expect(clockParts(sunday, 'en').date).toBe('Sunday, March 22');
	});

	test('la del globo lleva el año', () => {
		expect(clockParts(sunday, 'es').longDate).toBe('domingo, 22 de marzo de 2026');
	});

	test('un idioma que Intl no acepta no tira abajo el reloj', () => {
		expect(() => clockParts(sunday, 'no es un idioma!!')).not.toThrow();
		expect(clockParts(sunday, '').time).toBe('09:05');
	});
});
