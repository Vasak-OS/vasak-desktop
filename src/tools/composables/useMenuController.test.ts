import { afterAll, beforeAll, describe, expect, test } from 'bun:test';
import { nextTick } from 'vue';
import { useMenuController } from '@/tools/composables/useMenuController';

/**
 * El estado compartido del menú (vasak-desktop#203): leer el menú, filtrar, las
 * categorías y los botones de sesión. Se prueba el módulo de verdad con un
 * `__TAURI_INTERNALS__` de mentira —`get_menu_items` contesta lo que la prueba
 * decide— sin montar ningún componente: el estado es de la llamada, no del
 * módulo, así que cada prueba arranca con un controlador limpio.
 */
const MENU = {
	all: {
		icon: 'applications-all',
		description: 'views.menu.all',
		apps: [
			{ name: 'Firefox', description: 'Navegador web', path: '/f', icon: 'firefox' },
			{ name: 'Gimp', description: 'Edición de imágenes', path: '/g', icon: 'gimp' },
		],
	},
	graphics: {
		icon: 'applications-graphics',
		description: 'views.menu.graphics',
		apps: [{ name: 'Gimp', description: 'Edición de imágenes', path: '/g', icon: 'gimp' }],
	},
};

let menuResponse: unknown = MENU;
let throwOnMenu = false;
const calls: string[] = [];

beforeAll(() => {
	(globalThis as any).__TAURI_INTERNALS__ = {
		metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
		transformCallback: () => 0,
		invoke: async (cmd: string) => {
			calls.push(cmd);
			if (cmd === 'get_menu_items') {
				if (throwOnMenu) throw new Error('falló la lectura');
				return menuResponse;
			}
			if (cmd.startsWith('plugin:i18n')) return cmd.endsWith('get_locale') ? 'es' : {};
			return '';
		},
	};
});

afterAll(() => {
	delete (globalThis as any).__TAURI_INTERNALS__;
});

describe('el controlador del menú', () => {
	test('carga el menú y expone categorías y aplicaciones', async () => {
		menuResponse = MENU;
		throwOnMenu = false;
		const c = useMenuController();

		await c.setMenu();

		expect(c.isMenuEmpty.value).toBe(false);
		expect(c.menuData.value).toEqual(MENU);
		// La categoría por omisión es «all».
		expect(c.appsOfCategory.value).toEqual(MENU.all.apps);
		expect(c.categoryEntries.value.all?.[0]).toBe('all');
		expect(c.categoryEntries.value.others.map(([key]) => key)).toEqual(['graphics']);

		// Otra categoría cambia la lista de aplicaciones.
		c.categorySelected.value = 'graphics';
		expect(c.appsOfCategory.value).toEqual(MENU.graphics.apps);
	});

	test('los cinco botones de sesión corren sin tirar nada', async () => {
		const c = useMenuController();
		expect(c.sessionActions.value).toHaveLength(5);
		for (const action of c.sessionActions.value) {
			expect(() => action.handler()).not.toThrow();
		}
		await nextTick();
	});

	test('filtrar busca por nombre y por descripción, y acota el índice', async () => {
		menuResponse = MENU;
		throwOnMenu = false;
		const c = useMenuController();
		await c.setMenu();

		c.selectedIndex.value = 4;
		c.filter.value = 'fire';
		await nextTick();
		expect(c.appsFiltred.value.map((a: any) => a.path)).toEqual(['/f']);
		// Cambiar el filtro vuelve el índice a 0.
		expect(c.selectedIndex.value).toBe(0);

		// Por descripción.
		c.filter.value = 'edición';
		await nextTick();
		expect(c.appsFiltred.value.map((a: any) => a.path)).toEqual(['/g']);

		// Sin resultados, el índice se acota.
		c.selectedIndex.value = 3;
		c.filter.value = 'zzz';
		await nextTick();
		expect(c.appsFiltred.value).toEqual([]);
		expect(c.selectedIndex.value).toBe(0);

		// Sin consulta, no filtra nada.
		c.filter.value = '';
		await nextTick();
		expect(c.appsFiltred.value).toEqual([]);
	});

	test('un menú vacío queda marcado como vacío', async () => {
		menuResponse = {};
		throwOnMenu = false;
		const c = useMenuController();
		await c.setMenu();
		expect(c.isMenuEmpty.value).toBe(true);
		expect(c.menuData.value).toEqual({});
	});

	test('un error al leer el menú también lo deja vacío, sin tirar', async () => {
		throwOnMenu = true;
		const c = useMenuController();
		await c.setMenu();
		expect(c.isMenuEmpty.value).toBe(true);
	});
});
