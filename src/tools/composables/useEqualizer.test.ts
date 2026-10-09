import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { mount } from '@vue/test-utils';
import { defineComponent, h, nextTick } from 'vue';
import { missingEqualizer } from '@/interfaces/equalizer';
import { GAIN_INTERVAL_MS, useEqualizer } from './useEqualizer';

/**
 * El ecualizador visto desde la interfaz: qué le pide a Tauri y qué hace con
 * lo que llega por `equalizer-changed`.
 *
 * Tauri se dobla en `window.__TAURI_INTERNALS__` —lo que lee
 * `@tauri-apps/api` al llamar—, sin `mock.module`: así el doble no queda en la
 * caché de módulos de las otras pruebas.
 */

const calls: Array<{ cmd: string; args: Record<string, unknown> }> = [];
const callbacks = new Map<number, (event: unknown) => void>();
const listeners = new Map<string, number>();
let nextCallback = 1;
let replies: Record<string, unknown> = {};

const ROCK = {
	service: true,
	frequencies: [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000],
	range: [-12, 12],
	presets: ['flat', 'rock'],
	preset: 'rock',
	gains: [4.5, 3.5, 1.5, -1, -2, -1, 1.5, 3, 4, 4.5],
	enabled: true,
	available: true,
	saved: true,
};

beforeAll(() => {
	(globalThis as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
		metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
		transformCallback: (callback: (event: unknown) => void) => {
			const id = nextCallback++;
			callbacks.set(id, callback);
			return id;
		},
		unregisterCallback: (id: number) => callbacks.delete(id),
		invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
			if (cmd === 'plugin:event|listen') {
				listeners.set(String(args.event), Number(args.handler));
				return Number(args.handler);
			}
			calls.push({ cmd, args });
			if (cmd in replies) {
				const reply = replies[cmd];
				if (reply instanceof Error) throw reply;
				return reply;
			}
			return null;
		},
	};
	(
		globalThis as unknown as { __TAURI_EVENT_PLUGIN_INTERNALS__: unknown }
	).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
		unregisterListener: () => {},
	};
});

afterAll(() => {
	delete (globalThis as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
	delete (globalThis as unknown as { __TAURI_EVENT_PLUGIN_INTERNALS__?: unknown })
		.__TAURI_EVENT_PLUGIN_INTERNALS__;
});

beforeEach(() => {
	calls.length = 0;
	replies = {};
});

const settle = async () => {
	for (let i = 0; i < 4; i++) {
		await nextTick();
		await Promise.resolve();
	}
};

/** Monta el composable dentro de un componente, que es donde vive. */
async function mountEqualizer() {
	let api!: ReturnType<typeof useEqualizer>;
	const view = mount(
		defineComponent({
			setup() {
				api = useEqualizer();
				return () => h('div');
			},
		})
	);
	await settle();
	return { view, api };
}

const sent = (cmd: string) => calls.filter((call) => call.cmd === cmd).map((call) => call.args);

describe('el ecualizador desde la interfaz', () => {
	test('arranca como no disponible y lee el estado del backend', async () => {
		replies.equalizer_state = ROCK;
		const { view, api } = await mountEqualizer();
		expect(api.state.value).toEqual(missingEqualizer());

		await api.load();
		expect(api.state.value.preset).toBe('rock');
		expect(api.state.value.service).toBe(true);
		view.unmount();
	});

	test('si leer falla, queda no disponible en vez de romperse', async () => {
		replies.equalizer_state = new Error('sin bus');
		const { view, api } = await mountEqualizer();
		await api.load();
		expect(api.state.value.service).toBe(false);
		view.unmount();
	});

	test('lo que llega por equalizer-changed reemplaza el estado', async () => {
		const { view, api } = await mountEqualizer();
		const handler = callbacks.get(listeners.get('equalizer-changed') ?? -1);
		expect(handler).toBeDefined();

		handler?.({ event: 'equalizer-changed', id: 1, payload: ROCK });
		await settle();
		expect(api.state.value.gains).toEqual(ROCK.gains);
		view.unmount();
	});

	test('mover una banda se ve en el acto, pasa a propio y se manda con el último valor', async () => {
		replies.equalizer_state = ROCK;
		const { view, api } = await mountEqualizer();
		await api.load();

		api.setGain(3, 2);
		api.setGain(3, 2.5);
		api.setGain(3, 3);
		expect(api.state.value.gains[3]).toBe(3);
		expect(api.state.value.preset).toBe('custom');
		expect(api.state.value.saved).toBe(false);

		await Bun.sleep(GAIN_INTERVAL_MS * 2);
		const gains = sent('equalizer_set_gain');
		// La primera en el acto; las dos de en medio se juntan en la última.
		expect(gains).toEqual([
			{ band: 3, gain: 2 },
			{ band: 3, gain: 3 },
		]);
		view.unmount();
	});

	test('elegir un perfil lo pide al backend, y un error no se propaga', async () => {
		const { view, api } = await mountEqualizer();
		await api.setPreset('jazz');
		expect(sent('equalizer_set_preset')).toEqual([{ preset: 'jazz' }]);

		replies.equalizer_set_preset = new Error('InvalidArgs');
		await expect(api.setPreset('nada')).resolves.toBeUndefined();
		view.unmount();
	});

	test('al desmontar se manda lo que quedaba pendiente', async () => {
		replies.equalizer_state = ROCK;
		const { view, api } = await mountEqualizer();
		await api.load();
		api.setGain(0, 1);
		api.setGain(0, 6);
		view.unmount();
		await settle();
		expect(sent('equalizer_set_gain').at(-1)).toEqual({ band: 0, gain: 6 });
	});
});
