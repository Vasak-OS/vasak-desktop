import { describe, expect, test } from 'bun:test';
import {
	allApps,
	isFavorite,
	type MenuApp,
	resolveFavorites,
	toggleFavorite,
} from '@/tools/menu-favorites';

const app = (path: string, name = path): MenuApp => ({
	path,
	name,
	description: name,
	icon: 'app',
});

describe('los favoritos del menú', () => {
	test('`isFavorite` dice si la ruta está en la lista', () => {
		expect(isFavorite(['/a', '/b'], '/a')).toBe(true);
		expect(isFavorite(['/a', '/b'], '/c')).toBe(false);
	});

	test('`toggleFavorite` agrega lo que no está y saca lo que sí, sin mutar', () => {
		const before = ['/a', '/b'];
		expect(toggleFavorite(before, '/c')).toEqual(['/a', '/b', '/c']);
		expect(toggleFavorite(before, '/a')).toEqual(['/b']);
		// No tocó la lista original.
		expect(before).toEqual(['/a', '/b']);
	});

	test('una ruta vacía no se agrega nunca', () => {
		expect(toggleFavorite(['/a'], '')).toEqual(['/a']);
	});

	test('`resolveFavorites` respeta el orden guardado y deja afuera lo que no existe', () => {
		const apps = [app('/a', 'A'), app('/b', 'B'), app('/c', 'C')];
		// Orden distinto al de las apps, y una ruta que ya no existe.
		const resolved = resolveFavorites(apps, ['/c', '/x', '/a']);
		expect(resolved.map((a) => a.path)).toEqual(['/c', '/a']);
	});

	test('`allApps` saca la lista plana del árbol del menú, tolerante', () => {
		expect(allApps({ all: { apps: [app('/a')] } }).map((a) => a.path)).toEqual(['/a']);
		expect(allApps(null)).toEqual([]);
		expect(allApps({})).toEqual([]);
		expect(allApps({ all: {} })).toEqual([]);
	});
});
