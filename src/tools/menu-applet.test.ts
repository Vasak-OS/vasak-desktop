import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * El menú de aplicaciones como applet: lo que lo une al backend.
 *
 * Son cadenas y archivos que nada comprueba al compilar. La ruta tiene que ser
 * la que arma `anchored_applet.rs` (`#/applets/<ruta>`), y lo que se hace al
 * lanzar una aplicación tiene que esconder el applet y no cerrar la ventana de
 * Tauri: dentro de una superficie de capa esa ventana es la vacía, y cerrarla se
 * lleva puesto el webview, así que la próxima apertura recargaba el menú.
 */

const src = join(import.meta.dir, '..');
const read = (...path: string[]) => readFileSync(join(src, ...path), 'utf8');

describe('el menú es un applet', () => {
	test('la vista vive en la ruta de los applets', () => {
		const routes = read('routes', 'index.ts');
		const applets = routes.slice(routes.indexOf("path: '/applets'"));

		expect(applets).toMatch(/path: 'menu', component: \(\) => import\('@\/views\/MenuView\.vue'\)/);
		// Y no más en la ruta vieja, que ya no pide nadie.
		expect(routes).not.toMatch(/path: '\/menu'/);
	});

	test('la precarga de datos sigue a la ruta nueva', () => {
		expect(read('tools', 'ipc.batch.ts')).toMatch(/'\/applets\/menu': \['get_menu_items'\]/);
	});

	test('la tabla del backend nombra la misma ruta', () => {
		const table = read('..', 'src-tauri', 'src', 'windows_apps', 'anchored_applet.rs');

		expect(table).toMatch(/AppletSpec \{ id: "menu", route: "menu",/);
	});
});

describe('lanzar una aplicación esconde el menú', () => {
	for (const file of [
		['components', 'buttons', 'AppMenuButton.vue'],
		// La fila (`AppMenuCard`) y el mosaico (`AppTile`) lanzan por
		// `useAppLauncher`, que es donde vive ahora el `dismissMenu` (vasak-desktop#203).
		['tools', 'composables', 'useAppLauncher.ts'],
		['views', 'MenuView.vue'],
	]) {
		test(file.at(-1) ?? '', () => {
			const source = read(...file);

			expect(source).toMatch(/dismissMenu\(\)/);
			expect(source).not.toMatch(/getCurrentWindow/);
			expect(source).not.toMatch(/\.close\(\)/);
		});
	}
});

describe('el panel', () => {
	const panel = read('views', 'PanelView.vue');

	test('manda el botón del menú al abrirlo', () => {
		expect(panel).toMatch(/toggleMenu\(menuButton\.value\)/);
		// El botón es la lupa, una píldora de la librería (vasak-desktop#151):
		// el `ref` es la instancia, y `anchorOf` le lee el `$el`.
		expect(panel).toMatch(/<PanelPill\s+ref="menuButton"/);
		expect(panel).toMatch(/menuButton\.value\?\.\$el/);
	});

	test('informa dónde quedó el botón al montarse y al cambiar de lado', () => {
		expect(panel).toMatch(/reportMenuButton\(position\.value, menuButton\.value\)/);
		expect(panel).toMatch(/watch\(position,/);
	});
});
