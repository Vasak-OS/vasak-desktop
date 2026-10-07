/**
 * La luz nocturna del centro de control (vasak-desktop#178): el mosaico con su
 * detalle, el botón redondo del estado A y el formulario del detalle.
 *
 * Los componentes, el componible y los servicios son los de verdad; lo que se
 * dobla es Tauri. El `invoke` contesta como el escritorio (el estado de la
 * unidad por D-Bus) y como el plugin de pantalla (la configuración), y anota
 * todo lo que le piden. Que encender y apagar llegue a la unidad correcta por
 * el bus de systemd lo prueba `src-tauri/src/night_light.rs` contra un bus
 * privado.
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { Slider } from '@vasakgroup/vue-libvasak';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { CONTROL_CENTER_TILES } from '../src/tools/control-center-tiles';
import {
	formatCoordinate,
	maxNightTemperature,
	parseCoordinate,
} from '../src/tools/night-light-form';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'night-light-doubles.ts');
const dom = useDom();
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

const defaultConfig = () => ({
	mode: 'manual',
	dayTemperature: 6500,
	nightTemperature: 4000,
	sunrise: '07:00',
	sunset: '20:00',
	latitude: null as number | null,
	longitude: null as number | null,
});

/** El escritorio y el plugin, vistos desde el frontend. */
const desktop = {
	state: { available: true, enabled: false },
	config: defaultConfig(),
	weather: null as { lat: number; lon: number } | null,
	/** Cada `invoke` que no es de eventos ni de idioma, en orden. */
	calls: [] as Array<{ cmd: string; args: Record<string, unknown> }>,
	listeners: new Map<string, number[]>(),
	callbacks: new Map<number, (event: unknown) => void>(),
	nextId: 3000,
	emit(event: string, payload: unknown) {
		for (const id of this.listeners.get(event) ?? []) this.callbacks.get(id)?.({ event, id, payload });
	},
	sets() {
		return this.calls.filter((c) => c.cmd === 'set_night_light_enabled').map((c) => c.args.enabled);
	},
	commands() {
		return this.calls.map((c) => c.cmd);
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
		if (cmd === 'plugin:event|listen') {
			const ids = desktop.listeners.get(args.event as string) ?? [];
			ids.push(args.handler as number);
			desktop.listeners.set(args.event as string, ids);
			return args.handler;
		}
		if (cmd === 'plugin:i18n|load_translations') return {};
		if (cmd === 'plugin:i18n|get_locale') return 'es';
		if (cmd.startsWith('plugin:event|')) return null;
		desktop.calls.push({ cmd, args });
		switch (cmd) {
			case 'get_night_light_state':
				return { ...desktop.state };
			case 'set_night_light_enabled': {
				const previous = desktop.state.enabled;
				desktop.state.enabled = args.enabled as boolean;
				return previous;
			}
			case 'apply_night_light':
				return desktop.state.enabled;
			case 'weather_place':
				return desktop.weather;
			case 'plugin:display-manager|get_night_light':
				return { available: desktop.state.available, configured: true, config: { ...desktop.config } };
			case 'plugin:display-manager|set_night_light':
				desktop.config = { ...(args.config as ReturnType<typeof defaultConfig>) };
				return { available: desktop.state.available, configured: true, config: { ...desktop.config } };
			default:
				return '';
		}
	};
});

beforeEach(() => {
	desktop.state = { available: true, enabled: false };
	desktop.config = defaultConfig();
	desktop.weather = null;
	desktop.calls = [];
});

const settle = async () => {
	for (let i = 0; i < 8; i++) {
		await nextTick();
		await new Promise((resolve) => setTimeout(resolve, 0));
	}
};

describe('el mosaico «Luz nocturna»', () => {
	let Tile: any;
	beforeAll(async () => {
		Tile = await loadComponent(dom.workdir(), 'src/components/controls/tiles/NightLightTile.vue', DOUBLES, 'NightLightTile');
	}, 60_000);

	test('refleja el estado de la unidad que ya tenía la sesión', async () => {
		desktop.state.enabled = true;
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		expect(view.find('[data-tile-main]').attributes('aria-pressed')).toBe('true');
		view.unmount();
	});

	test('el toque principal la enciende y la apaga en el escritorio', async () => {
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(desktop.sets()).toEqual([true]);
		expect(view.find('[data-tile-main]').attributes('aria-pressed')).toBe('true');

		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(desktop.sets()).toEqual([true, false]);
		expect(view.find('[data-tile-main]').attributes('aria-pressed')).toBe('false');
		view.unmount();
	});

	test('un cambio de la unidad llega por el evento, sin volver a preguntar', async () => {
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		const asked = desktop.commands().length;

		desktop.emit('night-light-changed', { available: true, enabled: true });
		await settle();
		expect(view.find('[data-tile-main]').attributes('aria-pressed')).toBe('true');

		desktop.emit('night-light-changed', { available: true, enabled: false });
		await settle();
		expect(view.find('[data-tile-main]').attributes('aria-pressed')).toBe('false');
		expect(desktop.commands().length).toBe(asked);
		view.unmount();
	});

	test('la flecha abre el detalle y no la cambia', async () => {
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		await view.find('[data-tile-detail]').trigger('click');
		await settle();
		expect(view.emitted('open')).toHaveLength(1);
		expect(desktop.sets()).toEqual([]);
		view.unmount();
	});

	test('sin wlsunset se ve no disponible y tocarla no pide nada', async () => {
		desktop.state = { available: false, enabled: false };
		const view = mount(Tile, { attachTo: document.body });
		await settle();
		expect(view.find('[data-unavailable="true"]').exists()).toBe(true);
		await view.find('[data-tile-main]').trigger('click');
		await settle();
		expect(desktop.sets()).toEqual([]);
		view.unmount();
	});
});

describe('el botón redondo del estado A', () => {
	let Toggle: any;
	beforeAll(async () => {
		Toggle = await loadComponent(dom.workdir(), 'src/components/controls/NightLightToggle.vue', DOUBLES, 'NightLightToggle');
	}, 60_000);

	test('alterna el mismo estado y lo dice en aria-pressed', async () => {
		const view = mount(Toggle, { attachTo: document.body });
		await settle();
		expect(view.find('button').attributes('aria-pressed')).toBe('false');
		await view.find('button').trigger('click');
		await settle();
		expect(desktop.sets()).toEqual([true]);
		expect(view.find('button').attributes('aria-pressed')).toBe('true');
		view.unmount();
	});

	test('sin wlsunset no ocupa lugar en la fila', async () => {
		desktop.state = { available: false, enabled: false };
		const view = mount(Toggle, { attachTo: document.body });
		await settle();
		expect(view.find('button').exists()).toBe(false);
		view.unmount();
	});

	test('va en la fila de interruptores, entre No molestar y el tema', () => {
		const view = read('src/views/ControlCenterView.vue');
		const row = view.slice(view.indexOf('data-quick-toggles'), view.indexOf('data-more'));
		const order = ['<NetworkControl', '<BluetoothControl', '<DoNotDisturbToggle', '<NightLightToggle', '<ThemeToggle'].map(
			(tag) => row.indexOf(tag)
		);
		expect(order.every((index) => index > 0)).toBe(true);
		expect([...order].sort((a, b) => a - b)).toEqual(order);
	});
});

describe('el detalle', () => {
	let Detail: any;
	beforeAll(async () => {
		Detail = await loadComponent(
			dom.workdir(),
			'src/components/areas/night-light/NightLightDetail.vue',
			DOUBLES,
			'NightLightDetail'
		);
	}, 60_000);

	test('lee la configuración del plugin', async () => {
		desktop.config.nightTemperature = 3300;
		const view = mount(Detail, { attachTo: document.body });
		await settle();
		expect(desktop.commands()).toContain('plugin:display-manager|get_night_light');
		expect(view.findComponent(Slider).props('modelValue')).toBe(3300);
		expect(view.find('[data-night-light-times]').exists()).toBe(true);
		view.unmount();
	});

	test('la temperatura se guarda con el plugin y después se aplica', async () => {
		const view = mount(Detail, { attachTo: document.body });
		await settle();
		desktop.calls = [];
		view.findComponent(Slider).vm.$emit('update:modelValue', 3000);
		await settle();
		expect(desktop.commands()).toEqual(['plugin:display-manager|set_night_light', 'apply_night_light']);
		expect(desktop.config.nightTemperature).toBe(3000);
		expect(desktop.config.sunset).toBe('20:00', 'el resto queda como estaba');
		view.unmount();
	});

	test('el deslizador no deja una noche igual o más fría que el día', async () => {
		const view = mount(Detail, { attachTo: document.body });
		await settle();
		expect(view.findComponent(Slider).props('max')).toBe(6400);
		view.unmount();
	});

	test('el horario fijo guarda la hora que se elige', async () => {
		const view = mount(Detail, { attachTo: document.body });
		await settle();
		const sunset = view.find('[data-night-light-sunset] input, input[data-night-light-sunset]');
		await sunset.setValue('21:30');
		await sunset.trigger('change');
		await settle();
		expect(desktop.config.sunset).toBe('21:30');
		expect(desktop.commands()).toContain('apply_night_light');
		view.unmount();
	});

	test('al atardecer toma las coordenadas del clima si no había', async () => {
		desktop.weather = { lat: -34.6, lon: -58.38 };
		const view = mount(Detail, { attachTo: document.body });
		await settle();
		// Las opciones van en orden: «al atardecer» primero, «horario fijo» después.
		const options = view.findAll('[data-night-light-mode] button');
		expect(options).toHaveLength(2);
		await options[0]?.trigger('click');
		await settle();
		expect(desktop.config).toMatchObject({ mode: 'location', latitude: -34.6, longitude: -58.38 });
		expect(view.find('[data-night-light-location]').exists()).toBe(true);
		view.unmount();
	});

	test('unas coordenadas fuera de rango no se guardan y lo dice', async () => {
		desktop.config = { ...defaultConfig(), mode: 'location', latitude: 10, longitude: 10 };
		const view = mount(Detail, { attachTo: document.body });
		await settle();
		desktop.calls = [];
		const latitude = view.find('[data-night-light-latitude] input, input[data-night-light-latitude]');
		await latitude.setValue('95');
		await latitude.trigger('change');
		await settle();
		expect(desktop.commands()).not.toContain('plugin:display-manager|set_night_light');
		expect(view.find('[data-night-light-error]').exists()).toBe(true);
		view.unmount();
	});

	test('sin wlsunset lo explica y no deja tocar nada', async () => {
		desktop.state = { available: false, enabled: false };
		const view = mount(Detail, { attachTo: document.body });
		await settle();
		expect(view.find('[data-night-light-missing]').exists()).toBe(true);
		expect(view.findComponent(Slider).props('disabled')).toBe(true);
		view.unmount();
	});
});

describe('las cuentas del formulario', () => {
	test('la noche llega hasta un paso por debajo del día', () => {
		expect(maxNightTemperature(6500)).toBe(6400);
		expect(maxNightTemperature(900)).toBe(1000);
	});

	test('las coordenadas aceptan coma decimal, vacío y nada fuera de rango', () => {
		expect(parseCoordinate('-34,6', 90)).toBe(-34.6);
		expect(parseCoordinate(' 12.5 ', 90)).toBe(12.5);
		expect(parseCoordinate('', 90)).toBeNull();
		expect(parseCoordinate('91', 90)).toBeUndefined();
		expect(parseCoordinate('-181', 180)).toBeUndefined();
		expect(parseCoordinate('norte', 90)).toBeUndefined();
		expect(parseCoordinate('1e2', 180)).toBeUndefined();
		expect(formatCoordinate(null)).toBe('');
		expect(formatCoordinate(-58.38)).toBe('-58.38');
	});
});

describe('el registro y los textos', () => {
	test('el mosaico va después de No molestar, con detalle', () => {
		const ids = CONTROL_CENTER_TILES.map((tile) => tile.id);
		expect(ids.indexOf('night-light')).toBe(ids.indexOf('do-not-disturb') + 1);
		expect(CONTROL_CENTER_TILES.find((tile) => tile.id === 'night-light')?.detail).toBeDefined();
	});

	test('los textos existen en los dos idiomas', () => {
		const used = [
			'src/components/controls/tiles/NightLightTile.vue',
			'src/components/controls/NightLightToggle.vue',
			'src/components/areas/night-light/NightLightDetail.vue',
		].flatMap((file) => [...read(file).matchAll(/'(components\.[\w.]+)'/g)].map((m) => m[1] as string));
		expect(used.length).toBeGreaterThan(10);
		for (const locale of ['es', 'en']) {
			const catalog = Bun.YAML.parse(read(`src-tauri/locales/${locale}.yml`)) as Record<string, unknown>;
			for (const key of used) {
				const value = key.split('.').reduce<unknown>((node, part) => (node as Record<string, unknown>)?.[part], catalog);
				expect(value, `${locale}: ${key}`).toBeString();
			}
		}
	});
});
