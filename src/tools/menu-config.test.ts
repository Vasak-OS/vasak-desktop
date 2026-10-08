import { describe, expect, test } from 'bun:test';
import {
	DEFAULT_HEADER_STRENGTH,
	DEFAULT_MENU_VARIANT,
	DEFAULT_MENU_WIDGET,
	DEFAULT_SEARCH_POSITION,
	mergeMenuSection,
	readFavorites,
	readMenuConfig,
} from '@/tools/menu-config';

/**
 * La lectura de la sección `menu` se hace tolerante: el archivo se edita a mano,
 * así que un valor que no sea de los que se conocen —o que no diga nada— cae al
 * de fábrica y nunca deja el menú sin dibujar. Nada se castea.
 */
describe('la configuración del menú', () => {
	test('sin la sección, todo sale de fábrica', () => {
		const config = readMenuConfig(undefined);
		expect(config.variant).toBe(DEFAULT_MENU_VARIANT);
		expect(config.widget).toBe(DEFAULT_MENU_WIDGET);
		expect(config.searchPosition).toBe(DEFAULT_SEARCH_POSITION);
		expect(config.showUser).toBe(true);
		expect(config.showSessionActions).toBe(true);
		expect(config.showPlaces).toBe(false);
		expect(config.favorites).toEqual([]);
		expect(config.showFavorites).toBe(false);
		expect(config.header).toBe('none');
		expect(config.headerImage).toBe('');
		expect(config.headerStrength).toBe(DEFAULT_HEADER_STRENGTH);
		expect(config.showGreeting).toBe(true);
		expect(config.showWeather).toBe(true);
	});

	test('lee los valores válidos tal cual', () => {
		const config = readMenuConfig({
			menu: {
				variant: 'grid',
				widget: 'clock',
				showUser: false,
				showSessionActions: false,
				searchPosition: 'bottom',
				showPlaces: true,
				favorites: ['/a.desktop', '/b.desktop'],
				showFavorites: true,
				header: 'hero',
				headerImage: '/fondo.png',
				headerStrength: 40,
				showGreeting: false,
				showWeather: false,
			},
		});
		expect(config.variant).toBe('grid');
		expect(config.widget).toBe('clock');
		expect(config.showUser).toBe(false);
		expect(config.searchPosition).toBe('bottom');
		expect(config.showPlaces).toBe(true);
		expect(config.favorites).toEqual(['/a.desktop', '/b.desktop']);
		expect(config.header).toBe('hero');
		expect(config.headerImage).toBe('/fondo.png');
		expect(config.headerStrength).toBe(40);
	});

	test('un valor desconocido cae al de fábrica, no tira abajo el menú', () => {
		const config = readMenuConfig({
			menu: { variant: 'isla', widget: 'gato', searchPosition: 'izquierda', header: 'banner' },
		});
		expect(config.variant).toBe(DEFAULT_MENU_VARIANT);
		expect(config.widget).toBe(DEFAULT_MENU_WIDGET);
		expect(config.searchPosition).toBe(DEFAULT_SEARCH_POSITION);
		expect(config.header).toBe('none');
	});

	test('un booleano mal escrito a mano no apaga una parte sin querer', () => {
		// Sólo un `true`/`false` de verdad cuenta.
		const config = readMenuConfig({ menu: { showUser: 'no', showWeather: 0 } });
		expect(config.showUser).toBe(true);
		expect(config.showWeather).toBe(true);
	});

	test('la fuerza del hero se acota a 0–100 y lo no numérico cae al de fábrica', () => {
		expect(readMenuConfig({ menu: { headerStrength: 150 } }).headerStrength).toBe(100);
		expect(readMenuConfig({ menu: { headerStrength: -20 } }).headerStrength).toBe(0);
		expect(readMenuConfig({ menu: { headerStrength: 'mucho' } }).headerStrength).toBe(
			DEFAULT_HEADER_STRENGTH
		);
		expect(readMenuConfig({ menu: { headerStrength: Number.NaN } }).headerStrength).toBe(
			DEFAULT_HEADER_STRENGTH
		);
	});

	test('la imagen del hero sólo se acepta si es un string', () => {
		expect(readMenuConfig({ menu: { headerImage: 42 } }).headerImage).toBe('');
	});
});

describe('la lista de favoritos', () => {
	test('lo que no es un arreglo da una lista vacía', () => {
		expect(readFavorites(undefined)).toEqual([]);
		expect(readFavorites('x')).toEqual([]);
		expect(readFavorites({})).toEqual([]);
	});

	test('deja sólo strings no vacíos, sin repetir y en orden', () => {
		expect(readFavorites(['/a', '', '  ', '/b', '/a', 3, null, '/c'])).toEqual(['/a', '/b', '/c']);
	});

	test('recorta los espacios de los bordes', () => {
		expect(readFavorites(['  /a.desktop  '])).toEqual(['/a.desktop']);
	});
});

describe('guardar preserva las claves ajenas', () => {
	test('mezcla sobre la sección cruda, dejando lo que no conoce', () => {
		const config = { menu: { variant: 'grid', desconocida: 42 }, otra: { x: 1 } };
		const merged = mergeMenuSection(config, { favorites: ['/a'] });
		expect(merged).toEqual({ variant: 'grid', desconocida: 42, favorites: ['/a'] });
	});

	test('sin sección previa arranca de un objeto vacío', () => {
		expect(mergeMenuSection(undefined, { showUser: false })).toEqual({ showUser: false });
		expect(mergeMenuSection({}, { showUser: false })).toEqual({ showUser: false });
	});

	test('parte de lo escrito, no de lo resuelto: no guarda los valores de fábrica', () => {
		// Si partiera de `readMenuConfig`, acá habría trece claves; parte de la
		// cruda, que sólo tiene la que estaba más la nueva.
		const merged = mergeMenuSection({ menu: { variant: 'tiles' } }, { widget: 'clock' });
		expect(Object.keys(merged).sort()).toEqual(['variant', 'widget']);
	});
});
