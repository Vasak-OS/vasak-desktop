import { describe, expect, test } from 'bun:test';
import { clampPage, gridTracks, pageCount } from '@/tools/menu-grid';

describe('el paginado de la grilla del menú', () => {
	test('cuenta columnas y filas que entran, con el hueco', () => {
		// 900 de ancho, columnas de 88 + 12 de hueco → floor((900+12)/100) = 9.
		// 400 de alto, filas de 92 + 12 → floor((400+12)/104) = 3.
		const t = gridTracks(900, 400, 88, 92, 12);
		expect(t.columns).toBe(9);
		expect(t.rows).toBe(3);
		expect(t.pageSize).toBe(27);
	});

	test('en angosto entran menos columnas y más páginas', () => {
		const t = gridTracks(360, 560, 88, 92, 12);
		expect(t.columns).toBe(3);
		expect(t.pageSize).toBe(t.columns * t.rows);
	});

	test('un área de alto o ancho cero nunca deja la página en cero', () => {
		const t = gridTracks(0, 0, 88, 92, 12);
		expect(t.columns).toBe(1);
		expect(t.rows).toBe(1);
		expect(t.pageSize).toBe(1);
	});

	test('la cuenta de páginas redondea hacia arriba y es al menos una', () => {
		expect(pageCount(40, 27)).toBe(2);
		expect(pageCount(27, 27)).toBe(1);
		expect(pageCount(0, 27)).toBe(1);
		// Un tamaño de página cero no divide por cero.
		expect(pageCount(10, 0)).toBe(10);
	});

	test('acota la página al rango válido', () => {
		expect(clampPage(5, 3)).toBe(2);
		expect(clampPage(-1, 3)).toBe(0);
		expect(clampPage(1, 3)).toBe(1);
		expect(clampPage(0, 1)).toBe(0);
	});
});
