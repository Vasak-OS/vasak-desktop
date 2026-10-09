import { describe, expect, test } from 'bun:test';
import { capitalizeFirst, clockParts } from './panel-clock';

describe('la píldora del reloj', () => {
	const sunday = new Date(2026, 2, 22, 9, 5, 0);

	test('la hora va con dos dígitos', () => {
		const parts = clockParts(sunday, 'es');
		expect(parts.time).toBe('09:05');
		expect([parts.hour, parts.minute]).toEqual(['09', '05']);
	});

	test('la fecha de abajo es la del día, en el idioma de la sesión y en mayúscula', () => {
		// En español `Intl` devuelve «domingo» y «22 de marzo» en minúscula: la
		// píldora los quiere empezados en mayúscula. En inglés ya vienen así.
		expect(clockParts(sunday, 'es').date).toBe('Domingo, 22 de marzo');
		expect(clockParts(sunday, 'en').date).toBe('Sunday, March 22');
	});

	test('el día y el mes de la fecha apilada empiezan en mayúscula', () => {
		const parts = clockParts(sunday, 'es');
		expect(parts.weekday).toBe('Domingo');
		expect(parts.dayMonth).toBe('22 de marzo');
	});

	test('la del globo lleva el año y también empieza en mayúscula', () => {
		expect(clockParts(sunday, 'es').longDate).toBe('Domingo, 22 de marzo de 2026');
	});

	test('un idioma que Intl no acepta no tira abajo el reloj', () => {
		expect(() => clockParts(sunday, 'no es un idioma!!')).not.toThrow();
		expect(clockParts(sunday, '').time).toBe('09:05');
	});
});

describe('capitalizeFirst', () => {
	test('pone en mayúscula sólo la primera letra, no cada palabra', () => {
		expect(capitalizeFirst('domingo')).toBe('Domingo');
		expect(capitalizeFirst('22 de marzo')).toBe('22 de marzo');
		expect(capitalizeFirst('lunes, 1 de enero')).toBe('Lunes, 1 de enero');
	});

	test('la cadena vacía queda igual y no se cae', () => {
		expect(capitalizeFirst('')).toBe('');
	});

	test('lo que ya empieza en mayúscula no cambia', () => {
		expect(capitalizeFirst('Sunday, March 22')).toBe('Sunday, March 22');
	});
});
