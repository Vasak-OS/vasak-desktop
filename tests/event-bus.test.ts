/**
 * El bus de eventos compartido: `listen()` es asíncrono, así que entre la
 * suscripción y el enganche nativo hay un hueco. `whenReady` deja a un consumidor
 * esperar el enganche antes de su lectura inicial. Sólo el borde de Tauri es de
 * mentira; el bus y `useSharedEvent` son los reales.
 */
import { afterAll, beforeAll, describe, expect, test } from 'bun:test';
import { mount } from '@vue/test-utils';
import { defineComponent, h, nextTick } from 'vue';
import { eventBus, useSharedEvent } from '../src/tools/event.bus';

const callbacks = new Map<number, (event: unknown) => void>();
const handlerOf = new Map<string, number>();
const gates = new Map<string, Promise<void>>();
let nextCallback = 1;

beforeAll(() => {
	(globalThis as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
		metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
		transformCallback: (callback: (event: unknown) => void) => {
			const id = nextCallback++;
			callbacks.set(id, callback);
			return id;
		},
		invoke: async (cmd: string, args: { event?: string; handler?: number }) => {
			if (cmd === 'plugin:event|listen' && args.event && args.handler !== undefined) {
				await gates.get(args.event);
				handlerOf.set(args.event, args.handler);
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

const settle = async () => {
	for (let i = 0; i < 6; i++) {
		await Promise.resolve();
		await nextTick();
	}
};

const gate = (event: string) => {
	let open!: () => void;
	gates.set(
		event,
		new Promise<void>((resolve) => {
			open = resolve;
		})
	);
	return open;
};

const emit = (event: string, payload: unknown) =>
	callbacks.get(handlerOf.get(event) ?? -1)?.({ event, id: 1, payload });

describe('el bus de eventos compartido', () => {
	test('whenReady no se resuelve hasta que el listener nativo engancha', async () => {
		const event = 'prueba-whenready';
		const open = gate(event);
		const off = eventBus.subscribe(event, () => {});
		let ready = false;
		void eventBus.whenReady(event).then(() => {
			ready = true;
		});
		await settle();
		expect(ready).toBe(false);

		open();
		await settle();
		expect(ready).toBe(true);
		off();
	});

	test('un evento que llega después del enganche alcanza al handler', async () => {
		const event = 'prueba-entrega';
		const open = gate(event);
		const received: unknown[] = [];
		const off = eventBus.subscribe(event, (payload) => received.push(payload));
		open();
		await eventBus.whenReady(event);

		emit(event, { n: 1 });

		expect(received).toEqual([{ n: 1 }]);
		off();
	});

	test('whenReady de un evento sin suscriptores resuelve enseguida', async () => {
		await expect(eventBus.whenReady('prueba-sin-nadie')).resolves.toBeUndefined();
	});

	test('useSharedEvent devuelve un ready que espera al enganche', async () => {
		const event = 'prueba-composable';
		const open = gate(event);
		let feed!: { ready: Promise<void> };
		const view = mount(
			defineComponent({
				setup() {
					feed = useSharedEvent(event, () => {});
					return () => h('div');
				},
			})
		);
		let ready = false;
		void feed.ready.then(() => {
			ready = true;
		});
		await settle();
		expect(ready).toBe(false);

		open();
		await settle();
		expect(ready).toBe(true);
		view.unmount();
	});
});
