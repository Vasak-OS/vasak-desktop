/**
 * El brillo por monitor y el perfil de energía del centro de control
 * (vasak-desktop#189): el deslizador del estado A, la lista de monitores y el
 * selector de perfil del estado B, y lo que deciden por debajo.
 *
 * Los componentes y los componibles son los de verdad, y también los clientes
 * publicados de los plugins (`@vasakgroup/plugin-display-manager` y
 * `@vasakgroup/plugin-power-profiles`); lo que se dobla es Tauri. El `invoke`
 * contesta como los dos plugins y anota lo que le piden, y los eventos se
 * disparan desde la prueba como los emitiría el plugin.
 */
import { afterEach, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount, type VueWrapper } from '@vue/test-utils';
import { nextTick } from 'vue';
import {
	createLatestWriter,
	ddcNotices,
	monitorName,
	primaryMonitor,
} from '../src/tools/display-brightness';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'display-power-doubles.ts');
const dom = useDom();
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

/**
 * Los textos se miran por su clave: el catálogo de idiomas es un único estado
 * del proceso y las pruebas no lo cargan. Que las claves existan en los dos
 * idiomas lo mira la última prueba.
 */
const BUILT_IN = 'components.MonitorBrightness.builtIn';
const PROFILE = 'components.PowerProfileControl.';

type Monitor = { output: string | null; kind: 'backlight' | 'ddc'; handle: string; percent: number };

const panel = (percent = 40): Monitor => ({
	output: 'eDP-1',
	kind: 'backlight',
	handle: 'intel_backlight',
	percent,
});
const hdmi = (percent = 70): Monitor => ({ output: 'HDMI-A-1', kind: 'ddc', handle: '5', percent });
const dp = (percent = 30): Monitor => ({ output: 'DP-2', kind: 'ddc', handle: '7', percent });

const ready = { state: 'ready', reason: null as string | null, unsupported: [] as string[] };

/** Los dos plugins, vistos desde el frontend. */
const desktop = {
	report: { monitors: [panel()] as Monitor[], ddc: { ...ready } },
	power: {
		available: true,
		profiles: ['power-saver', 'balanced', 'performance'],
		activeProfile: 'balanced' as string | null,
		performanceDegraded: null as string | null,
	},
	failPower: false,
	calls: [] as Array<{ cmd: string; args: Record<string, unknown> }>,
	listeners: new Map<string, number[]>(),
	callbacks: new Map<number, (event: unknown) => void>(),
	nextId: 5000,
	emit(event: string, payload: unknown) {
		for (const id of this.listeners.get(event) ?? []) this.callbacks.get(id)?.({ event, id, payload });
	},
	of(cmd: string) {
		return this.calls.filter((c) => c.cmd === cmd).map((c) => c.args);
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
		if (cmd === 'plugin:event|unlisten') {
			const ids = desktop.listeners.get(args.event as string) ?? [];
			desktop.listeners.set(
				args.event as string,
				ids.filter((id) => id !== args.eventId)
			);
			return null;
		}
		if (cmd === 'plugin:i18n|load_translations') return {};
		if (cmd === 'plugin:i18n|get_locale') return 'es';
		if (cmd.startsWith('plugin:event|')) return null;
		desktop.calls.push({ cmd, args });
		switch (cmd) {
			case 'plugin:display-manager|get_brightness':
				return structuredClone(desktop.report);
			case 'plugin:display-manager|set_brightness':
				return null;
			case 'plugin:power-profiles|get_power_state':
				return { ...desktop.power };
			case 'plugin:power-profiles|set_power_profile':
				if (desktop.failPower) throw new Error('el demonio dijo que no');
				desktop.power.activeProfile = args.profile as string;
				return { ...desktop.power };
			default:
				return '';
		}
	};
});

beforeEach(() => {
	desktop.report = { monitors: [panel()], ddc: { ...ready } };
	desktop.power = {
		available: true,
		profiles: ['power-saver', 'balanced', 'performance'],
		activeProfile: 'balanced',
		performanceDegraded: null,
	};
	desktop.failPower = false;
	desktop.calls = [];
});

const settle = async () => {
	for (let i = 0; i < 8; i++) {
		await nextTick();
		await new Promise((resolve) => setTimeout(resolve, 0));
	}
};

/** Lo que hace la persona: arrastrar (`input`) y soltar (`change`). */
async function drag(wrapper: VueWrapper, selector: string, values: number[]) {
	const input = wrapper.find<HTMLInputElement>(`${selector} input[type="range"]`);
	for (const value of values) {
		input.element.value = String(value);
		await input.trigger('input');
	}
	await settle();
}
async function release(wrapper: VueWrapper, selector: string) {
	await wrapper.find(`${selector} input[type="range"]`).trigger('change');
	await settle();
}

const sliderValue = (wrapper: VueWrapper, selector: string) =>
	Number(wrapper.find<HTMLInputElement>(`${selector} input[type="range"]`).element.value);

const views: VueWrapper[] = [];
afterEach(() => {
	for (const view of views.splice(0)) view.unmount();
});
const keep = (view: VueWrapper) => {
	views.push(view);
	return view;
};

describe('lo que decide el brillo, sin Vue', () => {
	test('el principal es el panel interno, y si no hay, el primer externo', () => {
		const report = { monitors: [hdmi(), panel()], ddc: ready } as never;
		expect(primaryMonitor(report)?.handle).toBe('intel_backlight');
		expect(primaryMonitor({ monitors: [dp(), hdmi()], ddc: ready } as never)?.handle).toBe('7');
		expect(primaryMonitor({ monitors: [], ddc: ready } as never)).toBeNull();
		expect(primaryMonitor(null)).toBeNull();
	});

	test('el panel se llama «Integrada» y los externos por su conector', () => {
		const monitors = [panel(), hdmi()] as never[];
		expect(monitorName(panel() as never, monitors, 'Integrada')).toBe('Integrada');
		expect(monitorName(hdmi() as never, monitors, 'Integrada')).toBe('HDMI-A-1');
		const twoPanels = [panel(), { ...panel(), output: 'eDP-2', handle: 'amdgpu_bl1' }] as never[];
		expect(monitorName(panel() as never, twoPanels, 'Integrada')).toBe('Integrada (eDP-1)');
	});

	test('sin DDC/CI se dice no disponible, con el motivo o el monitor', () => {
		expect(ddcNotices({ state: 'unavailable', reason: 'not-installed', unsupported: [] })).toEqual([
			{ key: 'components.MonitorBrightness.ddcNotInstalled', args: [], unavailable: true },
		]);
		expect(ddcNotices({ state: 'ready', reason: null, unsupported: ['DP-3'] })).toEqual([
			{ key: 'components.MonitorBrightness.ddcUnsupported', args: ['DP-3'], unavailable: true },
		]);
		expect(ddcNotices({ state: 'detecting', reason: null, unsupported: [] })[0]?.unavailable).toBe(false);
		expect(ddcNotices({ state: 'ready', reason: null, unsupported: [] })).toEqual([]);
	});

	test('mientras escribe, de lo que llega sólo se escribe el último', async () => {
		const written: Array<[string, number]> = [];
		const gates: Array<() => void> = [];
		const write = createLatestWriter<number>(
			(key, value) =>
				new Promise<void>((resolve) => {
					written.push([key, value]);
					gates.push(resolve);
				})
		);
		const done = write('ddc:5', 10);
		write('ddc:5', 20);
		write('ddc:5', 30);
		write('ddc:7', 90);
		expect(written).toEqual([
			['ddc:5', 10],
			['ddc:7', 90],
		]);
		gates.shift()?.();
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(written).toEqual([
			['ddc:5', 10],
			['ddc:7', 90],
			['ddc:5', 30],
		]);
		gates.splice(0).forEach((open) => {
			open();
		});
		await done;
	});

	test('una escritura que falla no se lleva puesta la última', async () => {
		const written: number[] = [];
		let first = true;
		const write = createLatestWriter<number>(async (_key, value) => {
			written.push(value);
			if (first) {
				first = false;
				await Promise.resolve();
				throw new Error('ocupado');
			}
		});
		const done = write('a', 1);
		write('a', 2);
		await done;
		expect(written).toEqual([1, 2]);
	});
});

describe('la lista del estado B: un deslizador por monitor', () => {
	let List: any;
	beforeAll(async () => {
		List = await loadComponent(
			dom.workdir(),
			'src/components/controls/MonitorBrightnessList.vue',
			DOUBLES,
			'MonitorBrightnessList'
		);
	}, 60_000);

	test('un control por monitor, con su nombre', async () => {
		desktop.report.monitors = [panel(40), hdmi(70), dp(30)];
		const view = keep(mount(List, { attachTo: document.body }));
		await settle();
		const rows = view.findAll('[data-monitor]');
		expect(rows.map((row) => row.attributes('data-monitor'))).toEqual([
			'backlight:intel_backlight',
			'ddc:5',
			'ddc:7',
		]);
		expect(view.findAll('[data-monitor-name]').map((name) => name.text())).toEqual([
			BUILT_IN,
			'HDMI-A-1',
			'DP-2',
		]);
		expect(sliderValue(view, '[data-monitor="ddc:5"]')).toBe(70);
	});

	test('con un solo monitor, un control y sin nombre', async () => {
		const view = keep(mount(List, { attachTo: document.body }));
		await settle();
		expect(view.findAll('input[type="range"]')).toHaveLength(1);
		expect(view.find('[data-monitor-name]').exists()).toBe(false);
	});

	test('arrastrar no escribe; soltar escribe sólo ese monitor y sólo el último valor', async () => {
		desktop.report.monitors = [panel(40), hdmi(70)];
		const view = keep(mount(List, { attachTo: document.body }));
		await settle();

		await drag(view, '[data-monitor="ddc:5"]', [60, 50, 45]);
		expect(desktop.of('plugin:display-manager|set_brightness')).toEqual([]);
		expect(sliderValue(view, '[data-monitor="ddc:5"]')).toBe(45);

		await release(view, '[data-monitor="ddc:5"]');
		expect(desktop.of('plugin:display-manager|set_brightness')).toEqual([
			{ kind: 'ddc', handle: '5', percent: 45 },
		]);
		expect(sliderValue(view, '[data-monitor="backlight:intel_backlight"]')).toBe(40);
	});

	test('un cambio llega por el evento del plugin, sin volver a preguntar', async () => {
		desktop.report.monitors = [panel(40), hdmi(70)];
		const view = keep(mount(List, { attachTo: document.body }));
		await settle();
		const asked = desktop.of('plugin:display-manager|get_brightness').length;

		desktop.emit('display-brightness-changed', { monitors: [panel(80), hdmi(70)], ddc: ready });
		await settle();
		expect(sliderValue(view, '[data-monitor="backlight:intel_backlight"]')).toBe(80);

		// Se conectó un monitor: aparece su deslizador.
		desktop.emit('display-brightness-changed', { monitors: [panel(80), hdmi(70), dp(10)], ddc: ready });
		await settle();
		expect(view.findAll('[data-monitor]')).toHaveLength(3);
		expect(desktop.of('plugin:display-manager|get_brightness').length).toBe(asked);
	});

	test('sin ddcutil, los monitores externos se ven no disponibles', async () => {
		desktop.report = {
			monitors: [panel()],
			ddc: { state: 'unavailable', reason: 'not-installed', unsupported: [] },
		};
		const view = keep(mount(List, { attachTo: document.body }));
		await settle();
		const notice = view.find('[data-ddc-notice][data-unavailable="true"]');
		expect(notice.exists()).toBe(true);
		expect(notice.text()).toBe('components.MonitorBrightness.ddcNotInstalled');
		// El panel sigue regulable, y ahora con su nombre: hay otro que no.
		expect(view.findAll('input[type="range"]')).toHaveLength(1);
		expect(view.find('[data-monitor-name]').text()).toBe(BUILT_IN);
	});

	test('un monitor que no contesta DDC/CI se nombra como no disponible', async () => {
		desktop.report = { monitors: [panel(), hdmi()], ddc: { ...ready, unsupported: ['DP-3'] } };
		const view = keep(mount(List, { attachTo: document.body }));
		await settle();
		const notice = view.find('[data-ddc-notice][data-unavailable="true"]');
		expect(notice.attributes('data-output')).toBe('DP-3');
		expect(notice.text()).toBe('components.MonitorBrightness.ddcUnsupported');
		expect(view.findAll('input[type="range"]')).toHaveLength(2);
	});

	test('sin ninguna pantalla que regular, «no disponible» y ningún deslizador', async () => {
		desktop.report = { monitors: [], ddc: { ...ready } };
		const view = keep(mount(List, { attachTo: document.body }));
		await settle();
		expect(view.find('input[type="range"]').exists()).toBe(false);
		expect(view.find('[data-unavailable="true"]').exists()).toBe(true);
	});

	test('al volver a abrir el centro se pregunta sólo si hay monitores externos', async () => {
		const view = keep(mount(List, { attachTo: document.body }));
		await settle();
		const asked = () => desktop.of('plugin:display-manager|get_brightness').length;
		const before = asked();
		desktop.emit('window-shown', null);
		await settle();
		expect(asked()).toBe(before);

		desktop.emit('display-brightness-changed', { monitors: [panel(), hdmi()], ddc: ready });
		await settle();
		desktop.emit('window-shown', null);
		await settle();
		expect(asked()).toBe(before + 1);
		view.unmount();
		views.splice(0);
	});
});

describe('el brillo del estado A: el monitor principal', () => {
	let Control: any;
	let List: any;
	beforeAll(async () => {
		Control = await loadComponent(
			dom.workdir(),
			'src/components/controls/BrightnessControl.vue',
			DOUBLES,
			'BrightnessControl'
		);
		List = await loadComponent(
			dom.workdir(),
			'src/components/controls/MonitorBrightnessList.vue',
			DOUBLES,
			'MonitorBrightnessListA'
		);
	}, 60_000);

	test('un solo deslizador, el del panel interno, aunque haya externos', async () => {
		desktop.report.monitors = [hdmi(70), panel(40)];
		const view = keep(mount(Control, { attachTo: document.body }));
		await settle();
		expect(view.findAll('input[type="range"]')).toHaveLength(1);
		expect(sliderValue(view, '[data-primary-brightness]')).toBe(40);

		await drag(view, '[data-primary-brightness]', [55]);
		await release(view, '[data-primary-brightness]');
		expect(desktop.of('plugin:display-manager|set_brightness')).toEqual([
			{ kind: 'backlight', handle: 'intel_backlight', percent: 55 },
		]);
	});

	test('sin panel interno, el primer monitor externo', async () => {
		desktop.report.monitors = [dp(30), hdmi(70)];
		const view = keep(mount(Control, { attachTo: document.body }));
		await settle();
		expect(sliderValue(view, '[data-primary-brightness]')).toBe(30);
	});

	test('sin nada que regular no ocupa lugar', async () => {
		desktop.report.monitors = [];
		const view = keep(mount(Control, { attachTo: document.body }));
		await settle();
		expect(view.find('[data-primary-brightness]').exists()).toBe(false);
	});

	test('A y B comparten una lectura y un oyente', async () => {
		desktop.report.monitors = [panel(40), hdmi(70)];
		const a = keep(mount(Control, { attachTo: document.body }));
		const b = keep(mount(List, { attachTo: document.body }));
		await settle();
		expect(desktop.of('plugin:display-manager|get_brightness')).toHaveLength(1);
		expect(desktop.listeners.get('display-brightness-changed')).toHaveLength(1);

		await drag(b, '[data-monitor="backlight:intel_backlight"]', [20]);
		await release(b, '[data-monitor="backlight:intel_backlight"]');
		expect(sliderValue(a, '[data-primary-brightness]')).toBe(20);
	});

	test('al irse el último componente, se suelta el oyente', async () => {
		const view = mount(Control, { attachTo: document.body });
		await settle();
		expect(desktop.listeners.get('display-brightness-changed')).toHaveLength(1);
		view.unmount();
		await settle();
		expect(desktop.listeners.get('display-brightness-changed')).toHaveLength(0);
	});
});

describe('el perfil de energía', () => {
	let Control: any;
	beforeAll(async () => {
		Control = await loadComponent(
			dom.workdir(),
			'src/components/controls/PowerProfileControl.vue',
			DOUBLES,
			'PowerProfileControl'
		);
	}, 60_000);

	const checked = (view: VueWrapper) =>
		view
			.findAll('[role="radio"]')
			.filter((radio) => radio.attributes('aria-checked') === 'true')
			.map((radio) => radio.text());

	test('Ahorro · Equilibrado · Rendimiento, con el activo marcado', async () => {
		const view = keep(mount(Control, { attachTo: document.body }));
		await settle();
		expect(view.findAll('[role="radio"]').map((radio) => radio.text())).toEqual([
			`${PROFILE}powerSaver`,
			`${PROFILE}balanced`,
			`${PROFILE}performance`,
		]);
		expect(checked(view)).toEqual([`${PROFILE}balanced`]);
	});

	test('refleja la señal del demonio sin volver a preguntar', async () => {
		const view = keep(mount(Control, { attachTo: document.body }));
		await settle();
		desktop.emit('power-profile-changed', { ...desktop.power, activeProfile: 'power-saver' });
		await settle();
		expect(checked(view)).toEqual([`${PROFILE}powerSaver`]);
		expect(desktop.of('plugin:power-profiles|get_power_state')).toHaveLength(1);
	});

	test('elegir uno lo cambia en el demonio', async () => {
		const view = keep(mount(Control, { attachTo: document.body }));
		await settle();
		await view.findAll('[role="radio"]')[2]?.trigger('click');
		await settle();
		expect(desktop.of('plugin:power-profiles|set_power_profile')).toEqual([{ profile: 'performance' }]);
		expect(checked(view)).toEqual([`${PROFILE}performance`]);
	});

	test('si el demonio lo rechaza, vuelve el que estaba', async () => {
		desktop.failPower = true;
		const view = keep(mount(Control, { attachTo: document.body }));
		await settle();
		await view.findAll('[role="radio"]')[0]?.trigger('click');
		await settle();
		expect(checked(view)).toEqual([`${PROFILE}balanced`]);
	});

	test('sin power-profiles-daemon se ve no disponible y no pide nada', async () => {
		desktop.power = { available: false, profiles: [], activeProfile: null, performanceDegraded: null };
		const view = keep(mount(Control, { attachTo: document.body }));
		await settle();
		expect(view.find('[data-power-profile]').attributes('data-unavailable')).toBe('true');
		expect(view.find('[data-power-status]').text()).toBe(`${PROFILE}unavailable`);
		const radios = view.findAll('[role="radio"]');
		expect(radios).toHaveLength(3);
		expect(radios.every((radio) => radio.attributes('disabled') !== undefined)).toBe(true);
		await radios[0]?.trigger('click');
		await settle();
		expect(desktop.of('plugin:power-profiles|set_power_profile')).toEqual([]);
	});

	test('si el demonio limitó el rendimiento, se dice', async () => {
		desktop.power.performanceDegraded = 'high-operating-temperature';
		const view = keep(mount(Control, { attachTo: document.body }));
		await settle();
		expect(view.find('[data-power-status]').text()).toBe(`${PROFILE}degraded`);
	});
});

describe('dónde van en el centro', () => {
	const view = read('src/views/ControlCenterView.vue');

	test('en B, debajo del volumen, y sólo después de ver B', () => {
		const box = view.slice(view.indexOf('data-display-power') - 200);
		expect(view.indexOf('<VolumeControl')).toBeLessThan(view.indexOf('<MonitorBrightnessList'));
		expect(view.indexOf('<MonitorBrightnessList')).toBeLessThan(view.indexOf('<PowerProfileControl'));
		expect(box).toContain('v-if="tilesMounted"');
		expect(box).toContain('v-show="showsSettings"');
	});

	test('en A, un solo brillo que se esconde en B', () => {
		const a = view.slice(view.indexOf('data-primary-brightness-box') - 60, view.indexOf('<VolumeControl'));
		expect(a).toContain('v-show="!showsSettings"');
		expect(a).toContain('<BrightnessControl');
	});
});

describe('los textos', () => {
	test('cada clave nueva está en español y en inglés', () => {
		const keys = [
			'title',
			'builtIn',
			'brightnessOf',
			'none',
			'ddcDetecting',
			'ddcNotInstalled',
			'ddcNoI2cDev',
			'ddcNoPermission',
			'ddcUnsupported',
		].map((key) => `MonitorBrightness.${key}`);
		keys.push(
			...['title', 'powerSaver', 'balanced', 'performance', 'unavailable', 'degraded'].map(
				(key) => `PowerProfileControl.${key}`
			)
		);
		for (const locale of ['es', 'en']) {
			const catalog = Bun.YAML.parse(read(`src-tauri/locales/${locale}.yml`)) as {
				components: Record<string, Record<string, string>>;
			};
			for (const key of keys) {
				const [group, name] = key.split('.') as [string, string];
				expect(typeof catalog.components[group]?.[name]).toBe('string');
			}
			expect(catalog.components.MonitorBrightness?.ddcUnsupported).toContain('{0}');
			expect(catalog.components.MonitorBrightness?.brightnessOf).toContain('{1}');
		}
	});
});
