/**
 * Modo avión en el centro de control (vasak-desktop#180): el mosaico, y lo que
 * hacen los de Wi-Fi y Bluetooth con una radio bloqueada.
 *
 * Los componibles son los de verdad; lo que se dobla es Tauri: el `invoke`
 * contesta la copia en memoria del escritorio, y la prueba dispara el evento
 * `airplane-mode-changed` como lo haría el kernel a través del escritorio. Qué
 * se bloquea y qué se restaura en `/dev/rfkill` se prueba del lado de Rust
 * (`src-tauri/src/airplane_mode.rs`).
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { CONTROL_CENTER_TILES } from '../src/tools/control-center-tiles';
import { loadComponent, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'airplane-mode-doubles.ts');
const dom = useDom();

const on = {
	available: true,
	enabled: true,
	hardware: false,
	wlanBlocked: true,
	bluetoothBlocked: true,
};
const off = { ...on, enabled: false, wlanBlocked: false, bluetoothBlocked: false };

/** El escritorio, visto desde el frontend: el estado y lo que le pidieron. */
const desktop = {
	state: { ...off },
	wireless: true,
	calls: [] as Array<{ cmd: string; args: unknown }>,
	listeners: new Map<string, number[]>(),
	callbacks: new Map<number, (event: unknown) => void>(),
	nextId: 1000,
	emit(event: string, payload: unknown) {
		for (const id of this.listeners.get(event) ?? []) this.callbacks.get(id)?.({ event, id, payload });
	},
	asked(cmd: string) {
		return this.calls.filter((call) => call.cmd === cmd);
	},
};

beforeAll(() => {
	const internals = (globalThis as unknown as { __TAURI_INTERNALS__: Record<string, unknown> })
		.__TAURI_INTERNALS__;
	internals.transformCallback = (callback: (event: unknown) => void) => {
		const id = desktop.nextId++;
		desktop.callbacks.set(id, callback);
		return id;
	};
	internals.invoke = async (cmd: string, args: Record<string, unknown>) => {
		desktop.calls.push({ cmd, args });
		if (cmd === 'get_airplane_mode') return { ...desktop.state };
		if (cmd === 'set_airplane_mode') {
			const previous = desktop.state.enabled;
			desktop.state = args.enabled ? { ...on } : { ...off };
			return previous;
		}
		if (cmd === 'unblock_radios') return null;
		if (cmd === 'plugin:network-manager|is_wireless_available') return true;
		if (cmd === 'plugin:network-manager|get_wireless_enabled') return desktop.wireless;
		if (cmd === 'plugin:network-manager|set_wireless_enabled') return null;
		if (cmd === 'plugin:event|listen') {
			const ids = desktop.listeners.get(args.event as string) ?? [];
			ids.push(args.handler as number);
			desktop.listeners.set(args.event as string, ids);
			return args.handler;
		}
		if (cmd === 'plugin:i18n|load_translations') return {};
		if (cmd === 'plugin:i18n|get_locale') return 'es';
		return '';
	};
});

beforeEach(() => {
	desktop.state = { ...off };
	desktop.wireless = true;
	desktop.calls = [];
});

const settle = async () => {
	for (let i = 0; i < 8; i++) {
		await nextTick();
		await new Promise((resolve) => setTimeout(resolve, 0));
	}
};

const pressed = (view: { find: (s: string) => { attributes: (a: string) => string | undefined } }) =>
	view.find('[data-tile-main]').attributes('aria-pressed');

describe('el mosaico «Modo avión»', () => {
	let Tile: any;
	beforeAll(async () => {
		Tile = await loadComponent(dom.workdir(), 'src/components/controls/tiles/AirplaneModeTile.vue', DOUBLES, 'AirplaneModeTile');
	}, 60_000);

	test('está en los ajustes, sin detalle', () => {
		const spec = CONTROL_CENTER_TILES.find((tile) => tile.id === 'airplane-mode');
		expect(spec).toBeDefined();
		expect(spec?.detail).toBeUndefined();
	});

	test('refleja el estado que ya tenía el escritorio', async () => {
		desktop.state = { ...on };
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		expect(pressed(view)).toBe('true');
		view.unmount();
	});

	test('tocarlo lo pone, y volver a tocarlo lo quita', async () => {
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		expect(pressed(view)).toBe('false');

		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(pressed(view)).toBe('true');
		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(pressed(view)).toBe('false');
		expect(desktop.asked('set_airplane_mode').map((call) => (call.args as { enabled: boolean }).enabled)).toEqual([
			true,
			false,
		]);
		view.unmount();
	});

	test('la tecla de avión llega por el evento, sin preguntar', async () => {
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		const reads = desktop.asked('get_airplane_mode').length;

		desktop.emit('airplane-mode-changed', on);
		await settle();
		expect(pressed(view)).toBe('true');
		expect(desktop.asked('get_airplane_mode').length).toBe(reads);
		expect(desktop.asked('set_airplane_mode')).toEqual([]);
		view.unmount();
	});

	test('con una radio bloqueada por el equipo lo dice', async () => {
		desktop.state = { ...on, hardware: true };
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		expect(view.text()).toContain('airplaneModeHardware');
		view.unmount();
	});

	test('sin radios se ve no disponible y no pide nada', async () => {
		desktop.state = { ...off, available: false };
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		expect(view.find('[data-unavailable="true"]').exists()).toBe(true);
		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(desktop.asked('set_airplane_mode')).toEqual([]);
		view.unmount();
	});
});

describe('los mosaicos de Wi-Fi y Bluetooth con la radio bloqueada', () => {
	let Network: any;
	let Bluetooth: any;
	let Airplane: any;
	beforeAll(async () => {
		Network = await loadComponent(dom.workdir(), 'src/components/controls/tiles/NetworkTile.vue', DOUBLES, 'NetworkTileAirplane');
		Bluetooth = await loadComponent(dom.workdir(), 'src/components/controls/tiles/BluetoothTile.vue', DOUBLES, 'BluetoothTileAirplane');
		Airplane = await loadComponent(dom.workdir(), 'src/components/controls/tiles/AirplaneModeTile.vue', DOUBLES, 'AirplaneModeTileShared');
	}, 60_000);

	test('el Wi-Fi se ve apagado aunque NetworkManager todavía diga que no', async () => {
		const view = mount(Network, { attachTo: document.body });
		await settle();
		expect(pressed(view)).toBe('true');

		desktop.emit('airplane-mode-changed', on);
		await settle();
		expect(pressed(view)).toBe('false');
		view.unmount();
		// El primer montaje carga la librería de la red: lleva más que el resto.
	}, 30_000);

	test('tocar el Wi-Fi bloqueado lo desbloquea, en vez de apagarlo en NetworkManager', async () => {
		desktop.state = { ...on };
		const view = mount(Network, { attachTo: document.body });
		await settle();
		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(desktop.asked('unblock_radios').map((call) => call.args)).toEqual([{ kind: 'wlan' }]);
		expect(desktop.asked('plugin:network-manager|set_wireless_enabled')).toEqual([]);
		view.unmount();
	});

	test('al desbloquearse se relee la radio', async () => {
		desktop.state = { ...on };
		const view = mount(Network, { attachTo: document.body });
		await settle();
		const reads = desktop.asked('plugin:network-manager|get_wireless_enabled').length;
		desktop.emit('airplane-mode-changed', off);
		await settle();
		expect(desktop.asked('plugin:network-manager|get_wireless_enabled').length).toBe(reads + 1);
		expect(pressed(view)).toBe('true');
		view.unmount();
	});

	test('el Bluetooth bloqueado se ve apagado, y tocarlo lo desbloquea', async () => {
		desktop.state = { ...on };
		const view = mount(Bluetooth, { attachTo: document.body });
		await settle();
		expect(pressed(view)).toBe('false');
		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(desktop.asked('unblock_radios').map((call) => call.args)).toEqual([{ kind: 'bluetooth' }]);
		expect(desktop.calls.some((call) => call.cmd.startsWith('plugin:bluetooth-manager|'))).toBe(false);
		view.unmount();
	});

	test('los tres mosaicos montados juntos preguntan el estado una sola vez', async () => {
		const views = [Network, Bluetooth, Airplane].map((tile) => mount(tile, { attachTo: document.body }));
		await settle();
		expect(desktop.asked('get_airplane_mode').length).toBe(1);
		for (const view of views) view.unmount();
	});
});
