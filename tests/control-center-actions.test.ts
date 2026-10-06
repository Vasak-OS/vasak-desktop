/**
 * Lo que comparten los botones redondos y los mosaicos del centro de control
 * (vasak-desktop#175): abrir el tablero de tiempo de pantalla, abrir el
 * lanzador y alternar el tema. Se prueba contra un `__TAURI_INTERNALS__` de
 * mentira (`support/mount-sfc.ts`), mirando lo que se le pidió al backend.
 */
import { beforeEach, describe, expect, test } from 'bun:test';
import { mount } from '@vue/test-utils';
import { createPinia } from 'pinia';
import { defineComponent, h, nextTick } from 'vue';
import { useThemeToggle } from '../src/tools/composables/useThemeToggle';
import { openScreenTime, openSearch } from '../src/tools/control-center-actions';
import { capitalizeFirst } from '../src/tools/text-case';
import { tauriCalls, useDom } from './support/mount-sfc';

useDom();

type Internals = { invoke: (cmd: string, args?: unknown) => Promise<unknown> };
const internals = () => (globalThis as unknown as { __TAURI_INTERNALS__: Internals }).__TAURI_INTERNALS__;

/** Cambia el `invoke` de mentira mientras dura `run`, y lo devuelve después. */
async function withInvoke(fake: Internals['invoke'], run: () => Promise<void>): Promise<void> {
	const original = internals().invoke;
	internals().invoke = fake;
	try {
		await run();
	} finally {
		internals().invoke = original;
	}
}

beforeEach(() => {
	tauriCalls.length = 0;
	document.documentElement.classList.remove('dark');
});

const commands = () => tauriCalls.map((call) => call.cmd);

describe('abrir lo que no vive en el centro', () => {
	test('el tablero de tiempo de pantalla: primero se va el centro', async () => {
		await openScreenTime();
		const list = commands();
		expect(list).toContain('hide_control_center');
		expect(list).toContain('toggle_applet');
		expect(list.indexOf('hide_control_center')).toBeLessThan(list.indexOf('toggle_applet'));
		expect(tauriCalls.find((call) => call.cmd === 'toggle_applet')?.args).toMatchObject({ applet: 'screen-time' });
	});

	test('si el backend falla, no rompe: lo anota y sigue', async () => {
		await withInvoke(async () => {
			throw new Error('sin backend');
		}, async () => {
			await expect(openScreenTime()).resolves.toBeUndefined();
			await expect(openSearch()).resolves.toBeUndefined();
		});
	});

	test('el lanzador se pide con --toggle, para no dejar un proceso por clic', async () => {
		await openSearch();
		const spawn = tauriCalls.find((call) => call.cmd === 'plugin:shell|spawn');
		expect(spawn?.args).toMatchObject({ program: 'vasak-prism', args: ['--toggle'] });
	});
});

describe('alternar el tema', () => {
	function host() {
		let api: ReturnType<typeof useThemeToggle> | undefined;
		const Host = defineComponent({
			setup() {
				api = useThemeToggle();
				return () => h('div');
			},
		});
		const view = mount(Host, { global: { plugins: [createPinia()] } });
		if (!api) throw new Error('sin composable');
		return { view, api };
	}

	test('sin configuración leída, es el claro y el icono es la luna', () => {
		const { view, api } = host();
		expect(api.isDark.value).toBe(false);
		expect(api.themeIcon.value).toBe('weather-clear-night');
		view.unmount();
	});

	test('pasa al oscuro al instante y se lo pide al plugin', async () => {
		const { view, api } = host();
		await nextTick();
		await api.toggleTheme();
		expect(document.documentElement.classList.contains('dark')).toBe(true);
		expect(tauriCalls.find((call) => call.cmd === 'plugin:config-manager|set_darkmode')?.args).toEqual({
			darkmode: true,
		});
		// Mientras cambia, un segundo toque no hace nada.
		expect(api.isSwitching.value).toBe(true);
		tauriCalls.length = 0;
		await api.toggleTheme();
		expect(commands()).not.toContain('plugin:config-manager|set_darkmode');
		await Bun.sleep(850);
		expect(api.isSwitching.value).toBe(false);
		view.unmount();
	});

	test('si el plugin falla, vuelve atrás', async () => {
		const { view, api } = host();
		await nextTick();
		await withInvoke(async () => {
			throw new Error('no se pudo escribir');
		}, () => api.toggleTheme());
		expect(document.documentElement.classList.contains('dark')).toBe(false);
		await Bun.sleep(850);
		view.unmount();
	});
});

describe('la primera letra en mayúscula', () => {
	test('sólo la primera: «Martes, 6 de octubre»', () => {
		expect(capitalizeFirst('martes, 6 de octubre', 'es')).toBe('Martes, 6 de octubre');
		expect(capitalizeFirst('tuesday, October 6', 'en')).toBe('Tuesday, October 6');
	});

	test('vacío queda vacío, y respeta el idioma', () => {
		expect(capitalizeFirst('')).toBe('');
		expect(capitalizeFirst('istanbul', 'tr')).toBe('İstanbul');
	});
});
