/**
 * Las apps con una tarea en curso para la cápsula de actividad
 * (vasak-desktop#281): se cruzan las ventanas con lo que cada app publica por
 * `LauncherEntry`, y sólo cuentan las que van a mitad de camino.
 *
 * El composable es el real; sólo el borde de Tauri (`invoke` y los eventos) es
 * de mentira, y desde ahí la prueba empuja ventanas y avances.
 */
import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { mount } from '@vue/test-utils';
import { defineComponent, h, nextTick } from 'vue';
import { type ActivityProgress, useActivityProgress } from '../src/tools/composables/useActivityProgress';

type Win = { id: string; title: string; is_minimized: boolean; icon: string; app_id?: string };

let windows: Win[] = [];
let entries: Array<{ desktop_id: string; progress?: number }> = [];
const callbacks = new Map<number, (event: unknown) => void>();
const listeners = new Map<string, number>();
let nextCallback = 1;

const win = (id: string, app_id?: string): Win => ({
	id,
	title: `Ventana ${id}`,
	is_minimized: false,
	icon: `icono-${id}`,
	app_id,
});

beforeAll(() => {
	(globalThis as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
		metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
		transformCallback: (callback: (event: unknown) => void) => {
			const id = nextCallback++;
			callbacks.set(id, callback);
			return id;
		},
		invoke: async (cmd: string, args: { event?: string; handler?: number }) => {
			if (cmd === 'get_windows') return windows;
			if (cmd === 'get_launcher_entries') return entries;
			if (cmd === 'plugin:event|listen' && args.event && args.handler !== undefined) {
				listeners.set(args.event, args.handler);
				return args.handler;
			}
			return null;
		},
	};
	(globalThis as unknown as { __TAURI_EVENT_PLUGIN_INTERNALS__: unknown }).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
		unregisterListener: () => {},
	};
});

afterAll(() => {
	delete (globalThis as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
	delete (globalThis as unknown as { __TAURI_EVENT_PLUGIN_INTERNALS__?: unknown }).__TAURI_EVENT_PLUGIN_INTERNALS__;
});

beforeEach(() => {
	windows = [];
	entries = [];
	listeners.clear();
});

const settle = async () => {
	for (let i = 0; i < 6; i++) {
		await Promise.resolve();
		await nextTick();
	}
};

async function mountActivity() {
	let exposed!: ReturnType<typeof useActivityProgress>;
	const view = mount(
		defineComponent({
			setup() {
				exposed = useActivityProgress();
				return () => h('div');
			},
		})
	);
	await settle();
	return { view, activity: exposed };
}

const ids = (apps: ActivityProgress[]) => apps.map((app) => app.id);

describe('las apps con una tarea en curso', () => {
	test('quedan sólo las que van entre 0 y 1, la más avanzada primero', async () => {
		windows = [win('a', 'org.app.uno'), win('b', 'org.app.dos'), win('c', 'org.app.tres')];
		entries = [
			{ desktop_id: 'org.app.uno', progress: 0.2 },
			{ desktop_id: 'org.app.dos', progress: 0.9 },
			{ desktop_id: 'org.app.tres', progress: 0.5 },
		];
		const { view, activity } = await mountActivity();

		expect(ids(activity.progressApps.value)).toEqual(['b', 'c', 'a']);
		expect(activity.topProgress.value).toEqual({
			id: 'b',
			title: 'Ventana b',
			icon: 'icono-b',
			progress: 0.9,
		});
		view.unmount();
	});

	test('un avance en 0 o en 1 no es una tarea en curso', async () => {
		windows = [win('a', 'org.app.uno'), win('b', 'org.app.dos'), win('c', 'org.app.tres')];
		entries = [
			{ desktop_id: 'org.app.uno', progress: 0 },
			{ desktop_id: 'org.app.dos', progress: 1 },
			{ desktop_id: 'org.app.tres' },
		];
		const { view, activity } = await mountActivity();

		expect(activity.progressApps.value).toEqual([]);
		expect(activity.topProgress.value).toBeNull();
		view.unmount();
	});

	test('un avance sin ventana de esa app no aparece', async () => {
		windows = [win('a', 'org.app.uno'), win('sin-id')];
		entries = [
			{ desktop_id: 'org.app.otra', progress: 0.4 },
			{ desktop_id: 'org.app.uno' },
		];
		const { view, activity } = await mountActivity();

		expect(activity.progressApps.value).toEqual([]);
		view.unmount();
	});

	test('la ventana se cruza con su entrada por app_id, sin el dominio invertido ni mayúsculas', async () => {
		windows = [win('a', 'Org.Dolphin'), win('b', 'org.telegram.desktop')];
		entries = [
			{ desktop_id: 'dolphin', progress: 0.3 },
			{ desktop_id: 'org.telegram.desktop', progress: 0.6 },
		];
		const { view, activity } = await mountActivity();

		expect(ids(activity.progressApps.value)).toEqual(['b', 'a']);
		view.unmount();
	});

	test('lo que publica la app después se suma sin recargar', async () => {
		windows = [win('a', 'org.app.uno')];
		const { view, activity } = await mountActivity();
		expect(activity.topProgress.value).toBeNull();

		callbacks.get(listeners.get('launcher-entry-update') ?? -1)?.({
			event: 'launcher-entry-update',
			id: 1,
			payload: [{ desktop_id: 'org.app.uno', progress: 0.75 }],
		});
		await settle();

		expect(activity.topProgress.value?.id).toBe('a');
		expect(activity.topProgress.value?.progress).toBe(0.75);
		view.unmount();
	});
});
