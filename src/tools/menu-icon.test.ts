import { describe, expect, test } from 'bun:test';
import { DEFAULT_MENU_ICON, resolveMenuIcon } from './menu-icon';

describe('el icono del botón de menú', () => {
	test('sin la clave, o con algo que no es un nombre, es el de siempre', () => {
		for (const section of [
			undefined,
			null,
			{},
			{ menuIcon: '' },
			{ menuIcon: '  ' },
			{ menuIcon: 3 },
		]) {
			expect(resolveMenuIcon(section)).toBe(DEFAULT_MENU_ICON);
		}
	});

	test('con un nombre, es ese', () => {
		expect(resolveMenuIcon({ menuIcon: 'view-app-grid' })).toBe('view-app-grid');
	});
});
