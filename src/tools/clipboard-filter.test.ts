import { describe, expect, test } from 'bun:test';
import { type ClipboardEntry, filterEntries } from './clipboard-filter';

const ENTRIES: ClipboardEntry[] = [
	{ id: 3, text: 'Hola mundo' },
	{ id: 2, text: 'una contraseña cualquiera' },
	{ id: 1, text: 'MAYÚSCULAS y minúsculas' },
];

describe('el filtro del historial del portapapeles', () => {
	test('con la búsqueda vacía devuelve todo, en el mismo orden', () => {
		expect(filterEntries(ENTRIES, '')).toEqual(ENTRIES);
		// Sólo espacios es como vacío.
		expect(filterEntries(ENTRIES, '   ')).toEqual(ENTRIES);
	});

	test('filtra por el texto sin distinguir mayúsculas', () => {
		expect(filterEntries(ENTRIES, 'MUNDO').map((e) => e.id)).toEqual([3]);
		expect(filterEntries(ENTRIES, 'minúsculas').map((e) => e.id)).toEqual([1]);
	});

	test('sin coincidencias devuelve una lista vacía', () => {
		expect(filterEntries(ENTRIES, 'no está')).toEqual([]);
	});

	test('conserva el orden de las coincidencias', () => {
		const dos = [
			{ id: 2, text: 'texto uno' },
			{ id: 1, text: 'texto dos' },
		];
		expect(filterEntries(dos, 'texto').map((e) => e.id)).toEqual([2, 1]);
	});
});
