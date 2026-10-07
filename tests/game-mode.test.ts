/**
 * El mosaico «Juegos» del centro de control (vasak-desktop#181).
 *
 * Usa el componible de verdad; lo que se dobla es Tauri: el `invoke` contesta
 * como el escritorio (`get_game_mode` / `set_game_mode`) y la prueba dispara el
 * evento `game-mode-changed` como lo haría el escritorio al cambiar el modo.
 * Lo que el modo hace al entrar y salir vive en Rust, con sus pruebas.
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { CONTROL_CENTER_TILES } from '../src/tools/control-center-tiles';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'game-mode-doubles.ts');
const dom = useDom();
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

/** El escritorio, visto desde el frontend: el estado y lo que le pidieron. */
const desktop = {
	enabled: false,
	/** Sin el comando (un escritorio anterior), `get_game_mode` rechaza. */
	missing: false,
	/** El motivo con el que rechaza `set_game_mode`, o `null` si anda. */
	failure: null as string | null,
	sets: [] as boolean[],
	listeners: new Map<string, number[]>(),
	callbacks: new Map<number, (event: unknown) => void>(),
	nextId: 2000,
	emit(event: string, payload: unknown) {
		for (const id of this.listeners.get(event) ?? []) this.callbacks.get(id)?.({ event, id, payload });
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
		if (cmd === 'get_game_mode') {
			if (desktop.missing) throw 'Command get_game_mode not found';
			return desktop.enabled;
		}
		if (cmd === 'set_game_mode') {
			desktop.sets.push(args.enabled as boolean);
			if (desktop.failure) throw desktop.failure;
			const previous = desktop.enabled;
			desktop.enabled = args.enabled as boolean;
			return previous;
		}
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
	desktop.enabled = false;
	desktop.missing = false;
	desktop.failure = null;
	desktop.sets = [];
});

const settle = async () => {
	for (let i = 0; i < 8; i++) {
		await nextTick();
		await new Promise((resolve) => setTimeout(resolve, 0));
	}
};

describe('el mosaico «Juegos»', () => {
	let Tile: any;
	beforeAll(async () => {
		Tile = await loadComponent(dom.workdir(), 'src/components/controls/tiles/GameModeTile.vue', DOUBLES, 'GameModeTile');
	}, 60_000);

	const pressed = (view: ReturnType<typeof mount>) => view.find('[data-tile-main]').attributes('aria-pressed');

	test('refleja el estado que ya tenía el escritorio', async () => {
		desktop.enabled = true;
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		expect(pressed(view)).toBe('true');
		view.unmount();
	});

	test('tocarlo pide set_game_mode con el valor contrario, y otra vez lo apaga', async () => {
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		expect(pressed(view)).toBe('false');

		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(desktop.sets).toEqual([true]);
		expect(pressed(view)).toBe('true');

		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(desktop.sets).toEqual([true, false]);
		expect(pressed(view)).toBe('false');
		view.unmount();
	});

	test('un cambio hecho desde otro lado llega por el evento, sin preguntar', async () => {
		const view = mount(Tile, { attachTo: document.body });
		await settle();

		desktop.emit('game-mode-changed', true);
		await settle();
		expect(pressed(view)).toBe('true');

		desktop.emit('game-mode-changed', false);
		await settle();
		expect(pressed(view)).toBe('false');
		expect(desktop.sets).toEqual([]);
		view.unmount();
	});

	test('una lectura inicial que vuelve tarde no pisa un evento más nuevo', async () => {
		let answer: (value: unknown) => void = () => {};
		const internals = (globalThis as unknown as { __TAURI_INTERNALS__: Record<string, any> }).__TAURI_INTERNALS__;
		const invoke = internals.invoke;
		internals.invoke = (cmd: string, args: Record<string, unknown>) =>
			cmd === 'get_game_mode'
				? new Promise((resolve) => {
						answer = resolve;
					})
				: invoke(cmd, args);
		try {
			const view = mount(Tile, { attachTo: document.body });
			await settle();
			desktop.emit('game-mode-changed', true);
			await settle();
			answer(false);
			await settle();
			expect(pressed(view)).toBe('true');
			view.unmount();
		} finally {
			internals.invoke = invoke;
		}
	});

	test('si el cambio falla, queda en el estado real y la línea de estado lo dice', async () => {
		desktop.failure = 'wayfire: sin IPC';
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		const before = view.text();

		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(desktop.sets).toEqual([true]);
		expect(pressed(view)).toBe('false');
		expect(view.text()).not.toBe(before);
		expect(view.text()).toContain('components.ControlCenterTiles.gameModeFailed');
		// Sigue andando: el próximo toque vuelve a pedir, y si sale bien el aviso se va.
		desktop.failure = null;
		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(desktop.sets).toEqual([true, true]);
		expect(pressed(view)).toBe('true');
		expect(view.text()).not.toContain('components.ControlCenterTiles.gameModeFailed');
		view.unmount();
	});

	test('un evento después del fallo también saca el aviso', async () => {
		desktop.failure = 'falló';
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(view.text()).toContain('components.ControlCenterTiles.gameModeFailed');

		desktop.emit('game-mode-changed', true);
		await settle();
		expect(pressed(view)).toBe('true');
		expect(view.text()).not.toContain('components.ControlCenterTiles.gameModeFailed');
		view.unmount();
	});

	test('sin el comando en el escritorio se ve no disponible y no pide nada', async () => {
		desktop.missing = true;
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		expect(view.find('[data-unavailable="true"]').exists()).toBe(true);
		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(desktop.sets).toEqual([]);
		view.unmount();
	});
});

describe('el registro y los textos', () => {
	test('el mosaico va después de No molestar y antes del tema, sin detalle', () => {
		const ids = CONTROL_CENTER_TILES.map((tile) => tile.id);
		expect(ids).toContain('game-mode');
		expect(ids.indexOf('game-mode')).toBeGreaterThan(ids.indexOf('do-not-disturb'));
		expect(ids.indexOf('game-mode')).toBeLessThan(ids.indexOf('theme'));
		const spec = CONTROL_CENTER_TILES.find((tile) => tile.id === 'game-mode');
		expect(spec?.detail).toBeUndefined();
		expect(spec?.requires).toBeUndefined();
	});

	test('el icono sale del tema del sistema, por nombre', () => {
		const source = read('src/components/controls/tiles/GameModeTile.vue');
		expect(source).toContain('icon="input-gaming-symbolic"');
		expect(source).not.toContain('<svg');
		expect(source).not.toMatch(/^\s+detail$/m);
	});

	test('los textos existen en los dos idiomas', () => {
		for (const locale of ['es', 'en']) {
			const catalog = Bun.YAML.parse(read(`src-tauri/locales/${locale}.yml`)) as {
				components: Record<string, Record<string, string>>;
			};
			const c = catalog.components.ControlCenterTiles;
			expect(c?.gameMode, locale).toBeString();
			expect(c?.gameModeFailed, locale).toBeString();
			expect(c?.enabled, locale).toBeString();
			expect(c?.disabled, locale).toBeString();
			expect(c?.unavailable, locale).toBeString();
		}
	});
});
