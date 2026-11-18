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
import { useSharedEvent } from '../src/tools/event.bus';
import { type ActivityProgress, useActivityProgress } from '../src/tools/composables/useActivityProgress';

type Win = { id: string; title: string; is_minimized: boolean; icon: string; app_id?: string };

let windows: Win[] = [];
let entries: Array<{ desktop_id: string; progress?: number }> = [];
const callbacks = new Map<number, (event: unknown) => void>();
const listeners = new Map<string, number>();
let nextCallback = 1;
// Si hay una compuerta, el `listen` nativo no engancha hasta que se abre.
let listenGate: Promise<void> | null = null;
// Compuertas por evento, para engancharlos de a uno.
const eventGates = new Map<string, Promise<void>>();
let windowReads = 0;
let entryReads = 0;
// Respuestas encoladas a mano: cada lectura toma la siguiente, así una prueba
// decide en qué orden resuelven. Sin cola, se contesta con `windows`/`entries`.
let windowReplies: Array<Promise<Win[]>> = [];
let entryReplies: Array<Promise<Array<{ desktop_id: string; progress?: number }>>> = [];

const deferred = <T>() => {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((done) => {
		resolve = done;
	});
	return { promise, resolve };
};

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
			if (cmd === 'get_windows') {
				windowReads++;
				return windowReplies.shift() ?? windows;
			}
			if (cmd === 'get_launcher_entries') {
				entryReads++;
				return entryReplies.shift() ?? entries;
			}
			if (cmd === 'plugin:event|listen' && args.event && args.handler !== undefined) {
				await listenGate;
				await eventGates.get(args.event);
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
	listenGate = null;
	eventGates.clear();
	windowReads = 0;
	entryReads = 0;
	windowReplies = [];
	entryReplies = [];
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

const emitLauncher = (payload: unknown) =>
	callbacks.get(listeners.get('launcher-entry-update') ?? -1)?.({ event: 'launcher-entry-update', id: 1, payload });

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

describe('la lectura inicial espera al enganche de los listeners', () => {
	test('no lee hasta que los listeners engancharon, y lo que cambió en el hueco no se pierde', async () => {
		let openGate!: () => void;
		listenGate = new Promise<void>((resolve) => {
			openGate = resolve;
		});
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

		// Listeners sin enganchar: todavía no hay lectura inicial.
		expect(windowReads).toBe(0);
		expect(entryReads).toBe(0);

		// El estado cambia en el hueco (sin listener, ese evento se perdería).
		windows = [win('a', 'org.app.uno')];
		entries = [{ desktop_id: 'org.app.uno', progress: 0.4 }];
		openGate();
		await settle();

		expect(windowReads).toBe(1);
		expect(entryReads).toBe(1);
		expect(exposed.topProgress.value?.id).toBe('a');
		expect(exposed.topProgress.value?.progress).toBe(0.4);
		view.unmount();
	});

	test('espera a los dos feeds: con uno solo enganchado no lee, sea cual sea', async () => {
		for (const [first, second] of [
			['window-delta', 'launcher-entry-update'],
			['launcher-entry-update', 'window-delta'],
		]) {
			windowReads = 0;
			entryReads = 0;
			eventGates.clear();
			const firstGate = deferred<void>();
			const secondGate = deferred<void>();
			eventGates.set(first, firstGate.promise);
			eventGates.set(second, secondGate.promise);
			const view = mount(
				defineComponent({
					setup() {
						useActivityProgress();
						return () => h('div');
					},
				})
			);
			await settle();

			firstGate.resolve();
			await settle();
			expect(windowReads).toBe(0);
			expect(entryReads).toBe(0);

			secondGate.resolve();
			await settle();
			expect(windowReads).toBe(1);
			expect(entryReads).toBe(1);
			view.unmount();
		}
	});
});

describe('las lecturas que resuelven fuera de orden', () => {
	test('una lectura de ventanas vieja no pisa a la más nueva', async () => {
		const old = deferred<Win[]>();
		const recent = deferred<Win[]>();
		windowReplies = [old.promise, recent.promise];
		entries = [
			{ desktop_id: 'org.app.uno', progress: 0.3 },
			{ desktop_id: 'org.app.dos', progress: 0.6 },
		];
		const { view, activity } = await mountActivity();
		// Lectura de montar (vieja) en vuelo; el window-delta dispara la nueva.
		callbacks.get(listeners.get('window-delta') ?? -1)?.({ event: 'window-delta', id: 2, payload: {} });
		await settle();
		expect(windowReads).toBe(2);

		// Resuelve primero la nueva y después la vieja.
		recent.resolve([win('nueva', 'org.app.dos')]);
		await settle();
		old.resolve([win('vieja', 'org.app.uno')]);
		await settle();

		expect(ids(activity.progressApps.value)).toEqual(['nueva']);
		view.unmount();
	});

	test('un launcher-entry-update que llegó durante la lectura inicial no se pisa con la foto vieja', async () => {
		const snapshot = deferred<Array<{ desktop_id: string; progress?: number }>>();
		entryReplies = [snapshot.promise];
		windows = [win('a', 'org.app.uno')];
		const { view, activity } = await mountActivity();
		expect(entryReads).toBe(1);

		emitLauncher([{ desktop_id: 'org.app.uno', progress: 0.8 }]);
		await settle();
		snapshot.resolve([{ desktop_id: 'org.app.uno', progress: 0.1 }]);
		await settle();

		expect(activity.topProgress.value?.progress).toBe(0.8);
		view.unmount();
	});
});

describe('el desmontaje durante la espera al enganche', () => {
	test('si se desmonta antes de que enganchen los listeners, no arranca ninguna lectura', async () => {
		const gate = deferred<void>();
		listenGate = gate.promise;
		// Otro suscriptor (el panel, en la app real) mantiene vivos los listeners
		// compartidos: así el enganche sí resuelve después de desmontar la cápsula.
		const keeper = mount(
			defineComponent({
				setup() {
					useSharedEvent('window-delta', () => {});
					useSharedEvent('launcher-entry-update', () => {});
					return () => h('div');
				},
			})
		);
		const { view } = await mountActivity();
		view.unmount();

		gate.resolve();
		await settle();

		expect(windowReads).toBe(0);
		expect(entryReads).toBe(0);
		keeper.unmount();
	});
});
