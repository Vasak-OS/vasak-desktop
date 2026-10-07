/**
 * La fila de sesión del centro de control (vasak-desktop#190, ancla #174).
 *
 * Bloquear, Cerrar sesión, Reiniciar, Apagar, en ese orden. Bloquear no
 * pregunta: pide `lock_screen`, que en el backend es `Session.Lock` de logind
 * por D-Bus (las pruebas de ese lado, contra un logind de mentira en un bus
 * privado, están en `src-tauri/src/session_lock.rs`). Las otras tres abren el
 * diálogo de sesión. Sin logind, Bloquear se ve no disponible y no pide nada.
 *
 * La fila se monta con los servicios de verdad y el `invoke` de mentira: lo
 * que se mira es qué comando le llega al backend.
 */
import { afterEach, beforeAll, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { CONFIRMED_ACTIONS, dialogAction, SESSION_ROW_ACTIONS } from '../src/tools/session-row';
import { logged } from './support/session-row-doubles';
import { loadComponent, ROOT, tauriCalls, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'session-row-doubles.ts');
const dom = useDom();
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

/** Lo que contesta el backend de mentira a lo de esta fila. */
const backend = { available: true as boolean | Error, lockFails: false };

type Internals = { invoke: (cmd: string, args: unknown) => Promise<unknown> };

const settle = async () => {
	for (let i = 0; i < 6; i++) await nextTick();
	await new Promise((resolve) => setTimeout(resolve, 0));
	for (let i = 0; i < 6; i++) await nextTick();
};

const commands = () => tauriCalls.map((call) => call.cmd).filter((cmd) => !cmd.startsWith('plugin:'));

let Row: any;
beforeAll(async () => {
	const internals = (globalThis as unknown as { __TAURI_INTERNALS__: Internals }).__TAURI_INTERNALS__;
	const original = internals.invoke;
	internals.invoke = async (cmd, args) => {
		if (cmd === 'lock_screen_available') {
			tauriCalls.push({ cmd, args });
			if (backend.available instanceof Error) throw backend.available;
			return backend.available;
		}
		if (cmd === 'lock_screen' && backend.lockFails) {
			tauriCalls.push({ cmd, args });
			// Después de fallar, logind ya no está.
			backend.available = false;
			throw new Error('logind no bloqueó la sesión');
		}
		return original(cmd, args);
	};
	Row = await loadComponent(dom.workdir(), 'src/components/areas/control-center/SessionActionsRow.vue', DOUBLES, 'SessionActionsRow');
}, 60_000);

afterEach(() => {
	tauriCalls.length = 0;
	logged.length = 0;
	backend.available = true;
	backend.lockFails = false;
	document.body.innerHTML = '';
});

async function mountRow() {
	const view = mount(Row, { attachTo: document.body });
	await settle();
	return view;
}

const lockButton = (view: Awaited<ReturnType<typeof mountRow>>) => view.find('[data-lock-action] button');
const confirmedButtons = (view: Awaited<ReturnType<typeof mountRow>>) =>
	view.findAll('[data-confirmed-actions] button');

describe('qué acciones van y en qué orden', () => {
	test('Bloquear, Cerrar sesión, Reiniciar, Apagar; Suspender queda en el diálogo', () => {
		expect([...SESSION_ROW_ACTIONS]).toEqual(['lock', 'logout', 'reboot', 'poweroff']);
		expect(SESSION_ROW_ACTIONS as readonly string[]).not.toContain('suspend');
	});

	test('sólo Bloquear se salta el diálogo', () => {
		expect([...CONFIRMED_ACTIONS]).toEqual(['logout', 'reboot', 'poweroff']);
		expect(dialogAction('lock')).toBeNull();
		expect(dialogAction('logout')).toBe('logout');
		expect(dialogAction('reboot')).toBe('reboot');
		// El diálogo dice `shutdown` donde la librería dice `poweroff`.
		expect(dialogAction('poweroff')).toBe('shutdown');
	});

	test('montada, los botones salen en ese orden', async () => {
		const view = await mountRow();
		const buttons = view.findAll('[data-session-actions] button');
		// El nombre de cada botón es su globo (`title`); el catálogo de la
		// prueba está vacío, así que se ve la clave.
		expect(buttons.map((button) => button.attributes('title'))).toEqual([
			'views.controlCenter.lock',
			'views.menu.logout',
			'views.menu.reboot',
			'views.menu.shutdown',
		]);
		view.unmount();
	});
});

describe('Bloquear', () => {
	test('pide el bloqueo al backend, sin diálogo, con el centro ya cerrado', async () => {
		const view = await mountRow();
		expect(commands()).toEqual(['lock_screen_available']);
		expect(lockButton(view).attributes('disabled')).toBeUndefined();

		await lockButton(view).trigger('click');
		await settle();

		expect(commands()).toEqual(['lock_screen_available', 'hide_control_center', 'lock_screen']);
		expect(commands()).not.toContain('toggle_session_popup');
		view.unmount();
	});

	test('sin logind se ve no disponible y no pide nada', async () => {
		backend.available = false;
		const view = await mountRow();

		const button = lockButton(view);
		expect(button.attributes('disabled')).toBeDefined();
		expect(button.attributes('title')).toBe('views.controlCenter.lockUnavailable');
		await button.trigger('click');
		await settle();
		expect(commands()).toEqual(['lock_screen_available']);
		// Las otras tres no dependen de logind para abrir su diálogo.
		for (const other of confirmedButtons(view)) expect(other.attributes('disabled')).toBeUndefined();
		view.unmount();
	});

	test('si preguntar falla, también se ve no disponible', async () => {
		backend.available = new Error('sin bus del sistema');
		const view = await mountRow();
		expect(lockButton(view).attributes('disabled')).toBeDefined();
		view.unmount();
	});

	test('si el bloqueo falla, lo anota y vuelve a preguntar: el botón deja de prometer', async () => {
		backend.lockFails = true;
		const view = await mountRow();

		await lockButton(view).trigger('click');
		await settle();

		expect(commands()).toEqual(['lock_screen_available', 'hide_control_center', 'lock_screen', 'lock_screen_available']);
		expect(logged.length).toBe(1);
		expect(lockButton(view).attributes('disabled')).toBeDefined();
		view.unmount();
	});
});

describe('las que preguntan', () => {
	test('cada una abre el diálogo de sesión con su acción, y ninguna bloquea', async () => {
		const view = await mountRow();
		for (const button of confirmedButtons(view)) await button.trigger('click');
		await settle();

		const dialogs = tauriCalls.filter((call) => call.cmd === 'toggle_session_popup').map((call) => call.args);
		expect(dialogs).toEqual([{ action: 'logout' }, { action: 'reboot' }, { action: 'shutdown' }]);
		expect(commands()).not.toContain('lock_screen');
		view.unmount();
	});
});

describe('el centro de control y el backend', () => {
	test('la vista usa la fila y ya no la lista de antes con Suspender', () => {
		const view = read('src/views/ControlCenterView.vue');
		expect(view).toContain('<SessionActionsRow');
		expect(view).not.toMatch(/'suspend'/);
	});

	test('los dos comandos están registrados en el backend', () => {
		const lib = read('src-tauri/src/lib.rs');
		expect(lib).toContain('session_lock::lock_screen,');
		expect(lib).toContain('session_lock::lock_screen_available,');
	});

	test('los textos existen en los dos idiomas', () => {
		for (const locale of ['es', 'en']) {
			const block = read(`src-tauri/locales/${locale}.yml`);
			const center = Bun.YAML.parse(block) as { views: { controlCenter: Record<string, string> } };
			expect(center.views.controlCenter.lock, locale).toBeString();
			expect(center.views.controlCenter.lockUnavailable, locale).toBeString();
		}
	});
});
