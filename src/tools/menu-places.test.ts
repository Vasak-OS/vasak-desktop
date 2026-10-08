import { describe, expect, test } from 'bun:test';
import type { UserDirectory } from '@/interfaces/file';
import { buildPlaces, TRASH_URI } from '@/tools/menu-places';

const dirs: UserDirectory[] = [
	{
		name: 'Documentos',
		icon: 'folder-documents',
		path: '/home/pato/Docs',
		xdgKey: 'XDG_DOCUMENTS_DIR',
	},
	{
		name: 'Descargas',
		icon: 'folder-download',
		path: '/home/pato/Baja',
		xdgKey: 'XDG_DOWNLOAD_DIR',
	},
];

describe('los lugares del menú', () => {
	test('son los cuatro fijos, con la papelera como ubicación virtual', () => {
		const places = buildPlaces('/home/pato', dirs);
		expect(places.map((p) => p.id)).toEqual(['home', 'documents', 'downloads', 'trash']);
		expect(places[0]?.path).toBe('/home/pato');
		expect(places[1]?.path).toBe('/home/pato/Docs');
		expect(places[2]?.path).toBe('/home/pato/Baja');
		expect(places[3]?.path).toBe(TRASH_URI);
	});

	test('sin la carpeta XDG, cae a la ruta de siempre bajo el home', () => {
		const places = buildPlaces('/home/pato', []);
		expect(places[1]?.path).toBe('/home/pato/Documents');
		expect(places[2]?.path).toBe('/home/pato/Downloads');
	});

	test('cada lugar trae su icono del tema y su clave de traducción', () => {
		const places = buildPlaces('/home/pato', dirs);
		expect(places.map((p) => p.icon)).toEqual([
			'user-home',
			'folder-documents',
			'folder-download',
			'user-trash',
		]);
		for (const place of places) {
			expect(place.labelKey).toMatch(/^views\.menu\.places\./);
		}
	});
});
