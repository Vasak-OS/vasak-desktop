/**
 * La búsqueda del menú queda enfocada cada vez que se abre (vasak-desktop#151).
 *
 * El menú se abre de dos maneras: el botón del lince en el panel y la tecla
 * Super (por D-Bus). Las dos terminan en el mismo lugar del backend
 * —`toggle_anchored_applet` con el applet `menu`—, que la primera vez crea la
 * superficie y las siguientes la vuelve a mostrar con el aviso `applet-shown`.
 * Acá se monta la vista de verdad, con el `SearchField` publicado, y se mira
 * adónde quedó el foco (`document.activeElement`) en los dos casos: la primera
 * apertura y cada reapertura.
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';
import { menu } from './support/menu-doubles';

const DOUBLES = join(import.meta.dir, 'support', 'menu-doubles.ts');
const dom = useDom();
let Menu: any;

beforeAll(async () => {
	Menu = await loadComponent(dom.workdir(), 'src/views/MenuView.vue', DOUBLES, 'MenuView');
}, 60_000);

beforeEach(() => menu.reset());

/** Los reintentos de foco son de 50 ms: se deja pasar más que eso. */
const settle = async () => {
	for (let i = 0; i < 4; i++) await nextTick();
	await new Promise((done) => setTimeout(done, 200));
	await nextTick();
};

const searchInput = (root: Element) => root.querySelector('input') as HTMLInputElement | null;

describe('la búsqueda del menú', () => {
	test('la primera apertura deja el foco en la búsqueda, aunque el campo nazca desactivado', async () => {
		const view = mount(Menu, { attachTo: document.body });
		await settle();

		const input = searchInput(view.element as Element);
		expect(input).not.toBeNull();
		expect(document.activeElement).toBe(input as Element);
		view.unmount();
	});

	test('cada reapertura vuelve a enfocarla y vacía lo que quedó escrito', async () => {
		const view = mount(Menu, { attachTo: document.body });
		await settle();
		const input = searchInput(view.element as Element) as HTMLInputElement;

		// Se escribió algo, el menú se fue y el foco quedó en otra parte.
		await view.find('input').setValue('fire');
		await settle();
		expect(input.value).toBe('fire');
		menu.leave();
		input.blur();
		const elsewhere = document.createElement('button');
		document.body.append(elsewhere);
		elsewhere.focus();
		expect(document.activeElement).toBe(elsewhere);

		// Vuelve a la vista (botón o tecla Super: el mismo aviso).
		menu.show();
		await settle();

		expect(document.activeElement).toBe(searchInput(view.element as Element) as Element);
		expect(searchInput(view.element as Element)?.value).toBe('');
		elsewhere.remove();
		view.unmount();
	});
});

describe('las dos maneras de abrir el menú llegan al mismo lugar', () => {
	const rust = (file: string) => readFileSync(join(ROOT, 'src-tauri/src', file), 'utf8');

	test('la tecla Super (D-Bus) abre con toggle_menu, igual que el botón del panel', () => {
		expect(rust('dbus_service.rs')).toMatch(/toggle_menu\(self\.app_handle\.clone\(\), None\)/);
		expect(rust('commands/menu.rs')).toMatch(/toggle_menu/);
	});

	test('y toggle_menu es el applet anclado `menu`, que avisa applet-shown al volver', () => {
		expect(rust('windows_apps/menu.rs')).toMatch(/toggle_anchored_applet\(app, MENU_APPLET, Some\(anchor\), size\)/);
		expect(rust('windows_apps/anchored_applet.rs')).toContain('emit("applet-shown"');
	});
});
