/**
 * «No molestar» en el escritorio (vasak-desktop#177): el mosaico del estado B,
 * el botón redondo del estado A y el indicador de la bandeja.
 *
 * Los tres usan el componible de verdad; lo que se dobla es Tauri: el `invoke`
 * contesta el estado de la copia en memoria del escritorio, y la prueba dispara
 * el evento `do-not-disturb-changed` como lo haría el demonio a través del
 * escritorio. El corte del cartel en sí vive en `vasak-flare-daemon`, con sus
 * pruebas.
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { CONTROL_CENTER_TILES } from '../src/tools/control-center-tiles';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'do-not-disturb-doubles.ts');
const dom = useDom();
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

/** El demonio, visto desde el frontend: el estado y lo que le pidieron. */
const daemon = {
	state: { available: true, enabled: false },
	sets: [] as boolean[],
	/** Los oyentes de eventos registrados por `listen`, por nombre. */
	listeners: new Map<string, number[]>(),
	callbacks: new Map<number, (event: unknown) => void>(),
	nextId: 1000,
	emit(event: string, payload: unknown) {
		for (const id of this.listeners.get(event) ?? []) this.callbacks.get(id)?.({ event, id, payload });
	},
};

beforeAll(() => {
	// Después del `beforeAll` de `useDom`: se reemplaza su `invoke` por uno que
	// sabe de «No molestar» y guarda los oyentes para poder dispararlos.
	const internals = (globalThis as unknown as { __TAURI_INTERNALS__: Record<string, unknown> })
		.__TAURI_INTERNALS__;
	internals.transformCallback = (callback: (event: unknown) => void) => {
		const id = daemon.nextId++;
		daemon.callbacks.set(id, callback);
		return id;
	};
	internals.invoke = async (cmd: string, args: Record<string, unknown>) => {
		if (cmd === 'get_do_not_disturb') return { ...daemon.state };
		if (cmd === 'set_do_not_disturb') {
			const previous = daemon.state.enabled;
			daemon.sets.push(args.enabled as boolean);
			daemon.state.enabled = args.enabled as boolean;
			return previous;
		}
		if (cmd === 'plugin:event|listen') {
			const ids = daemon.listeners.get(args.event as string) ?? [];
			ids.push(args.handler as number);
			daemon.listeners.set(args.event as string, ids);
			return args.handler;
		}
		if (cmd === 'plugin:i18n|load_translations') return {};
		if (cmd === 'plugin:i18n|get_locale') return 'es';
		return '';
	};
});

beforeEach(() => {
	daemon.state = { available: true, enabled: false };
	daemon.sets = [];
});

const settle = async () => {
	for (let i = 0; i < 8; i++) {
		await nextTick();
		await new Promise((resolve) => setTimeout(resolve, 0));
	}
};

describe('el mosaico de «No molestar»', () => {
	let Tile: any;
	beforeAll(async () => {
		Tile = await loadComponent(dom.workdir(), 'src/components/controls/tiles/DoNotDisturbTile.vue', DOUBLES, 'DoNotDisturbTile');
	}, 60_000);

	test('refleja el estado que ya tenía el escritorio', async () => {
		daemon.state.enabled = true;
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		expect(view.find('[data-tile-main]').attributes('aria-pressed')).toBe('true');
		view.unmount();
	});

	test('tocarlo lo cambia en el demonio, y vuelve a tocarlo lo deja como estaba', async () => {
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		expect(view.find('[data-tile-main]').attributes('aria-pressed')).toBe('false');

		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(daemon.sets).toEqual([true]);
		expect(view.find('[data-tile-main]').attributes('aria-pressed')).toBe('true');

		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(daemon.sets).toEqual([true, false]);
		expect(view.find('[data-tile-main]').attributes('aria-pressed')).toBe('false');
		view.unmount();
	});

	test('un cambio hecho desde otro lado llega por el evento, sin preguntar', async () => {
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		const asked = daemon.sets.length;

		daemon.emit('do-not-disturb-changed', { available: true, enabled: true });
		await settle();
		expect(view.find('[data-tile-main]').attributes('aria-pressed')).toBe('true');
		expect(daemon.sets.length).toBe(asked);
		view.unmount();
	});

	test('sin demonio que lo entienda se ve no disponible y no pide nada', async () => {
		daemon.state = { available: false, enabled: false };
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		expect(view.find('[data-unavailable="true"]').exists()).toBe(true);
		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(daemon.sets).toEqual([]);
		view.unmount();
	});
});

describe('el botón redondo del estado A', () => {
	let Toggle: any;
	beforeAll(async () => {
		Toggle = await loadComponent(dom.workdir(), 'src/components/controls/DoNotDisturbToggle.vue', DOUBLES, 'DoNotDisturbToggle');
	}, 60_000);

	test('alterna el mismo estado y lo dice en aria-pressed', async () => {
		const view = mount(Toggle, { attachTo: document.body });
		await settle();
		expect(view.find('button').attributes('aria-pressed')).toBe('false');
		await view.find('button').trigger('click');
		await settle();
		expect(daemon.sets).toEqual([true]);
		expect(view.find('button').attributes('aria-pressed')).toBe('true');
		view.unmount();
	});

	test('va en la fila de interruptores, entre Bluetooth y el tema', () => {
		const view = read('src/views/ControlCenterView.vue');
		const row = view.slice(view.indexOf('data-quick-toggles'), view.indexOf('data-more'));
		const order = ['<NetworkControl', '<BluetoothControl', '<DoNotDisturbToggle', '<ThemeToggle'].map((tag) =>
			row.indexOf(tag)
		);
		expect(order.every((index) => index > 0)).toBe(true);
		expect([...order].sort((a, b) => a - b)).toEqual(order);
	});
});

describe('el indicador de la bandeja', () => {
	let Indicator: any;
	beforeAll(async () => {
		Indicator = await loadComponent(dom.workdir(), 'src/components/buttons/TrayIconDoNotDisturb.vue', DOUBLES, 'TrayIconDoNotDisturb');
	}, 60_000);

	test('aparece sólo con el modo puesto, y sigue al evento', async () => {
		const view = mount(Indicator, { attachTo: document.body });
		await settle();
		expect(view.find('[data-do-not-disturb]').exists()).toBe(false);

		daemon.emit('do-not-disturb-changed', { available: true, enabled: true });
		await settle();
		expect(view.find('[data-do-not-disturb]').exists()).toBe(true);

		daemon.emit('do-not-disturb-changed', { available: true, enabled: false });
		await settle();
		expect(view.find('[data-do-not-disturb]').exists()).toBe(false);
		view.unmount();
	});

	test('está en la bandeja', () => {
		expect(read('src/components/areas/panel/TrayBarArea.vue')).toContain('<TrayIconDoNotDisturb');
	});
});

describe('el registro y los textos', () => {
	test('el mosaico va después de Bluetooth, sin detalle', () => {
		const ids = CONTROL_CENTER_TILES.map((tile) => tile.id);
		expect(ids.indexOf('do-not-disturb')).toBe(ids.indexOf('bluetooth') + 1);
		expect(CONTROL_CENTER_TILES.find((tile) => tile.id === 'do-not-disturb')?.detail).toBeUndefined();
	});

	test('los textos existen en los dos idiomas', () => {
		for (const locale of ['es', 'en']) {
			const catalog = Bun.YAML.parse(read(`src-tauri/locales/${locale}.yml`)) as {
				components: Record<string, Record<string, string>>;
			};
			const c = catalog.components;
			expect(c.ControlCenterTiles?.doNotDisturb, locale).toBeString();
			expect(c.ControlCenterTiles?.doNotDisturbOn, locale).toBeString();
			expect(c.ControlCenterTiles?.unavailable, locale).toBeString();
			expect(c.DoNotDisturbToggle?.label, locale).toBeString();
			expect(c.TrayIconDoNotDisturb?.label, locale).toBeString();
		}
	});
});
