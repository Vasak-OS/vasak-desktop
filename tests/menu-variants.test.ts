import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * El menú de inicio personalizable (vasak-desktop#203).
 *
 * Son cadenas y estructura que nada comprueba al compilar: que `MenuView` sea un
 * switch de esqueletos por `menu.variant`, que el compacto siga siendo el de
 * siempre con opciones que sólo agregan piezas, y que las piezas nuevas (hero,
 * lugares, favoritos) respeten las reglas de diseño —translúcidas, sin
 * `backdrop-blur`, con el widget elegible—. La guardia general de colores,
 * radios, iconos y breakpoints vive en `design-guard.test.ts`; acá se mira lo
 * propio de esta función.
 */
const ROOT = join(import.meta.dir, '..');
const read = (file: string) => readFileSync(join(ROOT, file), 'utf8');

describe('MenuView es un switch de esqueletos', () => {
	const view = read('src/views/MenuView.vue');

	test('resuelve la variante a un componente con `<component :is>`', () => {
		expect(view).toMatch(/<component\s+:is="layoutComponent"/);
		expect(view).toMatch(/const LAYOUTS[^=]*=/);
		expect(view).toMatch(/compact:\s*CompactMenu/);
	});

	test('las cinco variantes están mapeadas a su esqueleto', () => {
		for (const [variant, component] of [
			['compact', 'CompactMenu'],
			['classic', 'ClassicMenu'],
			['grid', 'GridMenu'],
			['favorites', 'FavoritesMenu'],
			['tiles', 'TilesMenu'],
		]) {
			expect(view).toMatch(new RegExp(`${variant}:\\s*${component}`));
		}
	});

	test('la variante sin esqueleto propio cae al compacto', () => {
		expect(view).toMatch(/LAYOUTS\[menu\.value\.variant\]\s*\?\?\s*CompactMenu/);
	});

	test('le pasa a la variante el controlador compartido y la config', () => {
		expect(view).toMatch(/:controller="controller"/);
		expect(view).toMatch(/:menu="menu"/);
	});

	test('el marco responde a las opciones de usuario, acciones y buscador', () => {
		expect(view).toMatch(/<UserMenuCard v-if="menu\.showUser"/);
		expect(view).toMatch(/<MenuSessionActions v-if="menu\.showSessionActions"/);
		expect(view).toMatch(/v-if="menu\.searchPosition === 'top'"/);
		expect(view).toMatch(/v-if="menu\.searchPosition === 'bottom'"/);
		expect(view).toMatch(/<MenuHero v-if="menu\.header === 'hero'"/);
	});
});

describe('la variante compacta', () => {
	const compact = read('src/components/areas/menu/layouts/CompactMenu.vue');

	test('el widget del hueco es elegible y `none` lo oculta', () => {
		expect(compact).toMatch(/widget !== 'none'/);
		expect(compact).toMatch(/<WidgetSlot[^>]*:type="widgetType"/);
		// Ya no está fijo en el clima.
		expect(compact).not.toMatch(/<WidgetSlot type="weather"/);
	});

	test('los favoritos y los lugares sólo aparecen si se piden', () => {
		expect(compact).toMatch(/<FavoritesArea v-if="showFavorites"/);
		expect(compact).toMatch(/<PlacesColumn v-if="showPlaces"/);
	});

	test('por omisión mantiene el formato de siempre (tres zonas)', () => {
		// La grilla de dos columnas a lo ancho y las filas del hueco de la
		// derecha son las de hoy: una regresión las cambiaría.
		expect(compact).toMatch(/@3xl:grid-cols-3/);
		expect(compact).toMatch(/grid-rows-\[auto_18rem\]/);
	});
});

describe('las piezas nuevas respetan el diseño', () => {
	test('el hero es translúcido, sin backdrop-blur, con la imagen por headerStrength', () => {
		const hero = read('src/components/areas/menu/MenuHero.vue');
		// Sólo la plantilla: la prosa del comentario nombra `backdrop-blur` a
		// propósito, para explicar que no se usa.
		const template = hero.slice(hero.indexOf('<template>'));
		expect(template).toContain('bg-ui-surface/70');
		expect(template).not.toMatch(/backdrop-blur/);
		expect(template).not.toMatch(/bg-ui-float|bg-ui-bg\b/);
		expect(template).toMatch(/opacity:\s*imageOpacity/);
		expect(template).toMatch(/v-if="menu\.showGreeting"/);
		expect(template).toMatch(/v-if="menu\.showWeather/);
	});

	test('los lugares son los fijos y abren con el gestor de archivos', () => {
		const places = read('src/components/areas/menu/PlacesColumn.vue');
		expect(places).toMatch(/buildPlaces/);
		expect(places).toMatch(/vasak-file-manager/);
	});

	test('los favoritos lanzan la app y el clic derecho la desfija', () => {
		const fav = read('src/components/areas/menu/FavoritesArea.vue');
		// Lanzar y fijar/desfijar salen del composable compartido.
		expect(fav).toMatch(/useAppLauncher/);
		expect(fav).toMatch(/toggleFavoriteFor/);
		expect(fav).toMatch(/data-fav-path/);
	});

	test('fijar se hace desde el menú contextual de la aplicación', () => {
		// La fila usa el composable; el menú contextual (fijar/desfijar) vive ahí.
		const card = read('src/components/cards/AppMenuCard.vue');
		expect(card).toMatch(/useAppLauncher/);
		expect(card).toMatch(/toggleFavoriteFor/);

		const launcher = read('src/tools/composables/useAppLauncher.ts');
		expect(launcher).toMatch(/showContextMenu/);
		expect(launcher).toMatch(/toggleFavoritePath/);
		expect(launcher).toMatch(/favorites\.pin|favorites\.unpin/);
	});
});

describe('las cuatro variantes nuevas', () => {
	const layout = (name: string) =>
		read(`src/components/areas/menu/layouts/${name}.vue`);

	test('reciben el controlador y la config, como el compacto', () => {
		for (const name of ['ClassicMenu', 'GridMenu', 'FavoritesMenu', 'TilesMenu']) {
			expect(layout(name)).toMatch(/defineProps<\{\s*controller: MenuController;\s*menu: MenuConfig\s*\}>/);
		}
	});

	test('son contenedores responsive, sin breakpoints de viewport', () => {
		// La guardia general ya prohíbe `sm:`/`md:`; acá se exige que adapten por
		// contenedor (`@container`) para la regla de una columna en angosto.
		for (const name of ['ClassicMenu', 'GridMenu', 'FavoritesMenu', 'TilesMenu']) {
			expect(layout(name)).toContain('@container');
		}
	});

	test('la clásica usa la barra de categorías y reutiliza la lista de apps', () => {
		const classic = layout('ClassicMenu');
		expect(classic).toMatch(/<ListRow[\s\S]*role="option"/);
		expect(classic).toMatch(/<MenuArea/);
	});

	test('la grilla pagina los mosaicos y marca la categoría elegida', () => {
		const grid = layout('GridMenu');
		expect(grid).toMatch(/<PageDots/);
		expect(grid).toMatch(/<AppTile/);
		expect(grid).toMatch(/v-model:categorySelected="categorySelected"/);
	});

	test('favoritos muestra la cuadrícula de favoritos y los lugares', () => {
		const fav = layout('FavoritesMenu');
		expect(fav).toMatch(/<FavoritesArea/);
		expect(fav).toMatch(/<PlacesColumn/);
	});

	test('mosaicos agrupa por categoría con los mosaicos grandes', () => {
		const tiles = layout('TilesMenu');
		expect(tiles).toMatch(/<SectionHeading/);
		expect(tiles).toMatch(/<AppTile[^>]*size="lg"/);
	});

	test('AppTile lanza y fija/desfija con el clic derecho', () => {
		const tile = read('src/components/areas/menu/AppTile.vue');
		expect(tile).toMatch(/dismissMenu/);
		expect(tile).toMatch(/toggleFavoritePath/);
		expect(tile).toMatch(/favorites\.pin|favorites\.unpin/);
	});
});

describe('guardar la config del menú preserva lo ajeno', () => {
	test('el composable mezcla con mergeMenuSection y conserva el resto del archivo', () => {
		const source = read('src/tools/composables/useMenuConfig.ts');
		expect(source).toMatch(/mergeMenuSection\(current, partial\)/);
		expect(source).toMatch(/\.\.\.current/);
	});
});
