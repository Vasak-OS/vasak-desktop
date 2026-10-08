import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { closeContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { createPinia, setActivePinia } from 'pinia';
import { useAppLauncher } from '@/tools/composables/useAppLauncher';
import type { MenuApp } from '@/tools/menu-favorites';

/**
 * Lanzar y fijar/desfijar una aplicación del menú (vasak-desktop#203). Se prueba
 * el composable de verdad con un `__TAURI_INTERNALS__` de mentira; el menú
 * contextual se dibuja y se cierra sin elegir nada (`closeContextMenu`) para que
 * la promesa no quede colgada.
 */
const app: MenuApp = {
	name: 'Firefox',
	description: 'Navegador web',
	path: '/usr/share/applications/firefox.desktop',
	icon: 'firefox',
};

const calls: string[] = [];
let throwOnOpen = false;

beforeAll(() => {
	(globalThis as any).__TAURI_INTERNALS__ = {
		metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
		transformCallback: () => 0,
		invoke: async (cmd: string) => {
			calls.push(cmd);
			if (cmd === 'open_app' && throwOnOpen) throw new Error('no se pudo abrir');
			if (cmd === 'plugin:config-manager|read_config') return '';
			if (cmd.startsWith('plugin:i18n')) return cmd.endsWith('get_locale') ? 'es' : {};
			if (cmd === 'plugin:event|listen') return 1;
			return '';
		},
	};
});

afterAll(() => {
	delete (globalThis as any).__TAURI_INTERNALS__;
});

beforeEach(() => {
	setActivePinia(createPinia());
	calls.length = 0;
	throwOnOpen = false;
});

describe('useAppLauncher', () => {
	test('lanzar abre la aplicación y esconde el menú', async () => {
		const { launch } = useAppLauncher();
		await launch(app);
		expect(calls).toContain('open_app');
		expect(calls).toContain('dismiss_applet');
	});

	test('si abrir falla, igual esconde el menú sin tirar', async () => {
		throwOnOpen = true;
		const { launch } = useAppLauncher();
		await launch(app);
		expect(calls).toContain('dismiss_applet');
	});

	test('el menú de fijar se abre y, cerrado sin elegir, no fija nada', async () => {
		const { toggleFavoriteFor } = useAppLauncher();
		let prevented = false;
		const event = {
			preventDefault() {
				prevented = true;
			},
			clientX: 10,
			clientY: 20,
		} as unknown as MouseEvent;

		const pending = toggleFavoriteFor(app, event);
		// Dejar que el menú se dibuje y después cerrarlo sin elegir.
		await new Promise((done) => setTimeout(done, 30));
		closeContextMenu();
		await pending;

		expect(prevented).toBe(true);
		// Cerrado sin elegir: no se escribió ninguna configuración.
		expect(calls).not.toContain('plugin:config-manager|write_config');
	});
});
