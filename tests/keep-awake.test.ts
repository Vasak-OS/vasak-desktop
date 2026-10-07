/**
 * El mosaico «Mantener despierto» del centro de control (vasak-desktop#179).
 *
 * Usa el componible de verdad; lo que se dobla es Tauri: el `invoke` contesta
 * como el escritorio (`get_keep_awake` / `set_keep_awake`) y la prueba dispara
 * el evento `keep-awake-changed` como lo haría el escritorio. El inhibidor de
 * logind vive en Rust (`src-tauri/src/keep_awake.rs`), con su logind de
 * mentira en un bus privado.
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { CONTROL_CENTER_TILES } from '../src/tools/control-center-tiles';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'keep-awake-doubles.ts');
const dom = useDom();
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

/** El escritorio, visto desde el frontend: el estado y lo que le pidieron. */
const desktop = {
	state: { available: true, enabled: false },
	/** El motivo con el que rechaza `set_keep_awake`, o `null` si anda. */
	failure: null as string | null,
	sets: [] as boolean[],
	listeners: new Map<string, number[]>(),
	callbacks: new Map<number, (event: unknown) => void>(),
	nextId: 3000,
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
		if (cmd === 'get_keep_awake') return { ...desktop.state };
		if (cmd === 'set_keep_awake') {
			desktop.sets.push(args.enabled as boolean);
			if (desktop.failure) throw desktop.failure;
			const previous = desktop.state.enabled;
			desktop.state.enabled = args.enabled as boolean;
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
	desktop.state = { available: true, enabled: false };
	desktop.failure = null;
	desktop.sets = [];
});

const settle = async () => {
	for (let i = 0; i < 8; i++) {
		await nextTick();
		await new Promise((resolve) => setTimeout(resolve, 0));
	}
};

describe('el mosaico «Mantener despierto»', () => {
	let Tile: any;
	beforeAll(async () => {
		Tile = await loadComponent(dom.workdir(), 'src/components/controls/tiles/KeepAwakeTile.vue', DOUBLES, 'KeepAwakeTile');
	}, 60_000);

	const pressed = (view: ReturnType<typeof mount>) => view.find('[data-tile-main]').attributes('aria-pressed');

	test('refleja el estado que ya tenía el escritorio', async () => {
		desktop.state.enabled = true;
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		expect(pressed(view)).toBe('true');
		view.unmount();
	});

	test('tocarlo lo pone en el escritorio, y otra vez lo quita', async () => {
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

	test('un cambio hecho desde otra ventana llega por el evento, sin preguntar', async () => {
		const view = mount(Tile, { attachTo: document.body });
		await settle();

		desktop.emit('keep-awake-changed', { available: true, enabled: true });
		await settle();
		expect(pressed(view)).toBe('true');

		desktop.emit('keep-awake-changed', { available: true, enabled: false });
		await settle();
		expect(pressed(view)).toBe('false');
		expect(desktop.sets).toEqual([]);
		view.unmount();
	});

	test('si el cambio falla, queda en el estado real y la línea de estado lo dice', async () => {
		desktop.failure = 'logind no dio el inhibidor';
		const view = mount(Tile, { attachTo: document.body });
		await settle();

		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(desktop.sets).toEqual([true]);
		expect(pressed(view)).toBe('false');
		expect(view.text()).toContain('components.ControlCenterTiles.keepAwakeFailed');
		view.unmount();
	});

	test('sin logind se ve no disponible y no pide nada', async () => {
		desktop.state = { available: false, enabled: false };
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
	test('el mosaico va en el registro, sin detalle ni requisito', () => {
		const spec = CONTROL_CENTER_TILES.find((tile) => tile.id === 'keep-awake');
		expect(spec).toBeDefined();
		expect(spec?.detail).toBeUndefined();
		expect(spec?.requires).toBeUndefined();
	});

	test('el icono sale del tema del sistema, por nombre', () => {
		const source = read('src/components/controls/tiles/KeepAwakeTile.vue');
		expect(source).toContain("'caffeine-cup-full-symbolic'");
		expect(source).toContain("'caffeine-cup-empty-symbolic'");
		expect(source).not.toContain('<svg');
	});

	test('los textos existen en los dos idiomas, también el motivo que ve logind', () => {
		for (const locale of ['es', 'en']) {
			const catalog = Bun.YAML.parse(read(`src-tauri/locales/${locale}.yml`)) as {
				components: Record<string, Record<string, string>>;
			};
			const c = catalog.components.ControlCenterTiles;
			expect(c?.keepAwake, locale).toBeString();
			expect(c?.keepAwakeFailed, locale).toBeString();
			expect(c?.keepAwakeReason, locale).toBeString();
		}
		// La clave del motivo es la que busca Rust.
		expect(read('src-tauri/src/keep_awake.rs')).toContain('"components.ControlCenterTiles.keepAwakeReason"');
	});
});
