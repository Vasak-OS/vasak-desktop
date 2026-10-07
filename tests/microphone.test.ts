/**
 * El micrófono y la elección de dispositivos en el centro de control
 * (vasak-desktop#182).
 *
 * Los componentes usan el componible y los servicios de verdad; lo que se
 * dobla es Tauri: el `invoke` contesta como el escritorio —la fuente y el
 * sumidero por omisión, con su volumen y su silencio— y la prueba dispara
 * los eventos como los manda el applet de audio. Lo que lee `pw-dump` y lo
 * convierte en esos eventos tiene sus pruebas en Rust (`audio_input.rs`).
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { getMicrophoneIconName } from '../src/utils/volume';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'microphone-doubles.ts');
const dom = useDom();
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

interface Device {
	id: string;
	name: string;
	description: string;
	is_default: boolean;
	volume: number;
}

/** El audio del escritorio, visto desde el frontend. */
const audio = {
	microphone: null as null | { current: number; min: number; max: number; is_muted: boolean },
	inputs: [] as Device[],
	outputs: [] as Device[],
	calls: [] as Array<{ cmd: string; args: Record<string, unknown> }>,
	listeners: new Map<string, number[]>(),
	callbacks: new Map<number, (event: unknown) => void>(),
	nextId: 2000,
	emit(event: string, payload: unknown) {
		for (const id of this.listeners.get(event) ?? []) this.callbacks.get(id)?.({ event, id, payload });
	},
	choose(list: Device[], id: string) {
		for (const device of list) device.is_default = device.id === id;
	},
};

const device = (id: string, name: string, isDefault: boolean, volume = 0.5): Device => ({
	id,
	name,
	description: id,
	is_default: isDefault,
	volume,
});

beforeAll(() => {
	const internals = (globalThis as unknown as { __TAURI_INTERNALS__: Record<string, unknown> })
		.__TAURI_INTERNALS__;
	internals.transformCallback = (callback: (event: unknown) => void) => {
		const id = audio.nextId++;
		audio.callbacks.set(id, callback);
		return id;
	};
	internals.invoke = async (cmd: string, args: Record<string, unknown> = {}) => {
		audio.calls.push({ cmd, args });
		switch (cmd) {
			case 'get_microphone':
				return audio.microphone ? { ...audio.microphone } : null;
			case 'set_microphone_volume':
				if (audio.microphone) audio.microphone.current = args.volume as number;
				return null;
			case 'toggle_microphone_mute':
				if (!audio.microphone) throw new Error('sin micrófono');
				audio.microphone.is_muted = !audio.microphone.is_muted;
				return audio.microphone.is_muted;
			case 'get_audio_input_devices':
				return audio.inputs.map((d) => ({ ...d }));
			case 'set_audio_input_device':
				audio.choose(audio.inputs, args.deviceId as string);
				return true;
			case 'get_audio_devices':
				return audio.outputs.map((d) => ({ ...d }));
			case 'set_audio_device':
				audio.choose(audio.outputs, args.deviceId as string);
				return true;
			case 'get_audio_volume':
				return { current: 40, min: 0, max: 100, is_muted: false };
			case 'plugin:event|listen': {
				const ids = audio.listeners.get(args.event as string) ?? [];
				ids.push(args.handler as number);
				audio.listeners.set(args.event as string, ids);
				return args.handler;
			}
			case 'plugin:event|unlisten': {
				// Sin esto, un oyente de un componente ya desmontado seguiría
				// recibiendo y la cuenta de pedidos saldría multiplicada.
				const ids = audio.listeners.get(args.event as string) ?? [];
				audio.listeners.set(
					args.event as string,
					ids.filter((id) => id !== args.eventId)
				);
				return null;
			}
			case 'plugin:i18n|load_translations':
				return {};
			case 'plugin:i18n|get_locale':
				return 'es';
			default:
				return '';
		}
	};
});

beforeEach(() => {
	audio.microphone = { current: 47, min: 0, max: 100, is_muted: false };
	audio.inputs = [
		device('alsa_input.usb-mono', 'USB Advanced Audio Device Mono', true, 0.47),
		device('bluez_input.00_11_22', 'Auriculares WH-1000XM4', false, 0.5),
	];
	audio.outputs = [
		device('52', 'Audio interno Estéreo analógico', true, 0.6),
		device('80', 'Auriculares WH-1000XM4', false, 0.35),
	];
	audio.calls = [];
});

const settle = async () => {
	for (let i = 0; i < 8; i++) {
		await nextTick();
		await new Promise((resolve) => setTimeout(resolve, 0));
	}
};

const called = (cmd: string) => audio.calls.filter((call) => call.cmd === cmd);

describe('el control del micrófono', () => {
	let Control: any;
	beforeAll(async () => {
		Control = await loadComponent(dom.workdir(), 'src/components/controls/MicrophoneControl.vue', DOUBLES, 'MicrophoneControl');
	}, 60_000);

	test('refleja el volumen y el silencio de la fuente por omisión', async () => {
		audio.microphone = { current: 47, min: 0, max: 100, is_muted: true };
		const view = mount(Control, { attachTo: document.body });
		await settle();
		const slider = view.find('[data-microphone-slider] input[type="range"]');
		expect((slider.element as HTMLInputElement).value).toBe('47');
		expect(view.find('[data-microphone-slider]').text()).toContain('47%');
		// Silenciado: el botón dice que lo vuelve a activar.
		expect(view.find('[data-microphone-slider] button').attributes('aria-label')).toBe(
			'components.MicrophoneControl.unmute'
		);
		view.unmount();
	});

	test('el botón lo silencia en el sistema, y otra vez lo deja oír', async () => {
		const view = mount(Control, { attachTo: document.body });
		await settle();
		const button = () => view.find('[data-microphone-slider] button');
		expect(button().attributes('aria-label')).toBe('components.MicrophoneControl.mute');

		await button().trigger('click');
		await settle();
		expect(called('toggle_microphone_mute').length).toBe(1);
		expect(audio.microphone?.is_muted).toBe(true);
		expect(button().attributes('aria-label')).toBe('components.MicrophoneControl.unmute');

		await button().trigger('click');
		await settle();
		expect(audio.microphone?.is_muted).toBe(false);
		expect(button().attributes('aria-label')).toBe('components.MicrophoneControl.mute');
		view.unmount();
	});

	test('mover el deslizador cambia el volumen de la fuente, una vez por ráfaga', async () => {
		const view = mount(Control, { attachTo: document.body });
		await settle();
		const slider = view.find('[data-microphone-slider] input[type="range"]');
		for (const value of ['50', '60', '72']) await slider.setValue(value);
		await new Promise((resolve) => setTimeout(resolve, 120));
		expect(called('set_microphone_volume').map((call) => call.args.volume)).toEqual([72]);
		expect(audio.microphone?.current).toBe(72);
		view.unmount();
	});

	test('un cambio hecho desde otro lado llega por el evento, sin preguntar', async () => {
		const view = mount(Control, { attachTo: document.body });
		await settle();
		const asked = called('get_microphone').length;

		audio.emit('microphone-changed', { current: 15, min: 0, max: 100, is_muted: true });
		await settle();
		const slider = view.find('[data-microphone-slider] input[type="range"]');
		expect((slider.element as HTMLInputElement).value).toBe('15');
		expect(view.find('[data-microphone-slider] button').attributes('aria-label')).toBe(
			'components.MicrophoneControl.unmute'
		);
		expect(called('get_microphone').length).toBe(asked);
		view.unmount();
	});

	test('sin micrófono se ve no disponible, nunca roto, y la flecha sigue', async () => {
		audio.microphone = null;
		const view = mount(Control, { attachTo: document.body });
		await settle();
		expect(view.find('[data-microphone-slider]').exists()).toBe(false);
		expect(view.find('[data-microphone-unavailable]').text()).toContain('components.MicrophoneControl.unavailable');
		expect(view.find('[data-sheet-opener="audio-input"]').exists()).toBe(true);
		view.unmount();
	});

	test('la flecha pide abrir la elección de la entrada', async () => {
		const view = mount(Control, { attachTo: document.body });
		await settle();
		const arrow = view.find('[data-sheet-opener="audio-input"]');
		expect(arrow.attributes('aria-label')).toBe('components.MicrophoneControl.devices');
		await arrow.trigger('click');
		expect(view.emitted('open-devices')).toHaveLength(1);
		view.unmount();
	});
});

describe('elegir el dispositivo', () => {
	let Selector: any;
	beforeAll(async () => {
		Selector = await loadComponent(dom.workdir(), 'src/components/controls/AudioDeviceSelector.vue', DOUBLES, 'AudioDeviceSelector');
	}, 60_000);

	const checked = (view: any) =>
		view
			.findAll('[role="radio"]')
			.filter((radio: any) => radio.attributes('aria-checked') === 'true')
			.map((radio: any) => radio.text());

	test('la entrada: marca la fuente por omisión y elegir otra la cambia', async () => {
		const view = mount(Selector, { props: { kind: 'input' }, attachTo: document.body });
		await settle();
		expect(view.text()).toContain('components.AudioDeviceSelector.inputTitle');
		expect(checked(view).join()).toContain('USB Advanced Audio Device Mono');

		const headset = view.findAll('[role="radio"]').find((radio: any) => radio.text().includes('WH-1000XM4'));
		await headset?.trigger('click');
		await settle();
		expect(called('set_audio_input_device').map((call) => call.args.deviceId)).toEqual(['bluez_input.00_11_22']);
		expect(called('set_audio_device')).toEqual([]);
		expect(audio.inputs.find((d) => d.is_default)?.id).toBe('bluez_input.00_11_22');
		expect(checked(view).join()).toContain('WH-1000XM4');
		view.unmount();
	});

	test('la entrada sigue el evento de las entradas, no el de las salidas', async () => {
		const view = mount(Selector, { props: { kind: 'input' }, attachTo: document.body });
		await settle();
		audio.emit('audio-devices-changed', [device('99', 'Otra salida', true)]);
		await settle();
		expect(view.text()).not.toContain('Otra salida');

		audio.emit('audio-input-devices-changed', [device('usb-2', 'Micrófono de la cámara', true)]);
		await settle();
		expect(checked(view)).toHaveLength(1);
		expect(checked(view)[0]).toContain('Micrófono de la cámara');
		view.unmount();
	});

	test('de salida a entrada sin desmontarse: lista entradas y elegir cambia la entrada', async () => {
		const view = mount(Selector, { props: { kind: 'output' }, attachTo: document.body });
		await settle();
		expect(checked(view).join()).toContain('Audio interno');

		await view.setProps({ kind: 'input' });
		await settle();
		expect(view.text()).toContain('components.AudioDeviceSelector.inputTitle');
		expect(view.text()).not.toContain('Audio interno');
		expect(checked(view).join()).toContain('USB Advanced Audio Device Mono');

		const headset = view.findAll('[role="radio"]').find((radio: any) => radio.text().includes('WH-1000XM4'));
		await headset?.trigger('click');
		await settle();
		expect(called('set_audio_input_device').map((call) => call.args.deviceId)).toEqual(['bluez_input.00_11_22']);
		expect(called('set_audio_device')).toEqual([]);

		// Y el evento de las salidas ya no la toca.
		audio.emit('audio-devices-changed', [device('99', 'Otra salida', true)]);
		await settle();
		expect(view.text()).not.toContain('Otra salida');
		view.unmount();
	});

	test('la salida: elegir otra cambia el sumidero por omisión', async () => {
		const view = mount(Selector, { props: { kind: 'output' }, attachTo: document.body });
		await settle();
		expect(checked(view).join()).toContain('Audio interno');
		const headset = view.findAll('[role="radio"]').find((radio: any) => radio.text().includes('WH-1000XM4'));
		await headset?.trigger('click');
		await settle();
		expect(called('set_audio_device').map((call) => call.args.deviceId)).toEqual(['80']);
		expect(called('set_audio_input_device')).toEqual([]);
		expect(audio.outputs.find((d) => d.is_default)?.id).toBe('80');
		view.unmount();
	});
});

describe('el volumen de salida', () => {
	let Control: any;
	beforeAll(async () => {
		Control = await loadComponent(dom.workdir(), 'src/components/controls/VolumeControl.vue', DOUBLES, 'VolumeControl');
	}, 60_000);

	test('con `devices` gana la flecha que abre la elección de la salida', async () => {
		const view = mount(Control, { props: { devices: true }, attachTo: document.body });
		await settle();
		const arrow = view.find('[data-sheet-opener="audio-output"]');
		expect(arrow.attributes('aria-label')).toBe('components.VolumeControl.devices');
		await arrow.trigger('click');
		expect(view.emitted('open-devices')).toHaveLength(1);
		view.unmount();
	});

	test('sin `devices` no hay flecha', async () => {
		const view = mount(Control, { attachTo: document.body });
		await settle();
		expect(view.find('[data-sheet-opener]').exists()).toBe(false);
		view.unmount();
	});

	test('otra salida por omisión trae su volumen', async () => {
		const view = mount(Control, { attachTo: document.body });
		await settle();
		const before = called('get_audio_volume').length;
		audio.emit('audio-devices-changed', audio.outputs);
		await settle();
		expect(called('get_audio_volume').length).toBe(before + 1);
		view.unmount();
	});
});

describe('el icono del micrófono', () => {
	test('sale del tema por nombre, según el volumen y el silencio', () => {
		expect(getMicrophoneIconName(true, 80)).toBe('microphone-sensitivity-muted-symbolic');
		expect(getMicrophoneIconName(false, 0)).toBe('microphone-sensitivity-muted-symbolic');
		expect(getMicrophoneIconName(false, 20)).toBe('microphone-sensitivity-low-symbolic');
		expect(getMicrophoneIconName(false, 50)).toBe('microphone-sensitivity-medium-symbolic');
		expect(getMicrophoneIconName(false, 90)).toBe('microphone-sensitivity-high-symbolic');
	});
});

describe('en la vista', () => {
	const VIEW = read('src/views/ControlCenterView.vue');

	test('el micrófono va en B, debajo del volumen, y no se monta hasta ver B', () => {
		const template = VIEW.slice(VIEW.indexOf('<template>'));
		expect(template).toMatch(/<MicrophoneControl\s+v-if="tilesMounted"\s+v-show="showsSettings"/);
		expect(template.indexOf('<VolumeControl')).toBeLessThan(template.indexOf('<MicrophoneControl'));
		// Y arriba de la caja de brillo por monitor y energía (vasak-desktop#189).
		expect(template.indexOf('<MicrophoneControl')).toBeLessThan(template.indexOf('data-display-power'));
	});

	test('las dos flechas abren su ficha dentro del bloque, pasando a B', () => {
		expect(VIEW).toContain(`<VolumeControl devices @open-devices="openSheet('audio-output')" />`);
		expect(VIEW).toContain(`@open-devices="openSheet('audio-input')"`);
		const open = VIEW.slice(VIEW.indexOf('function openSheet'));
		expect(open.slice(0, 200)).toContain("mode.value = 'settings';");
		expect(open.slice(0, 200)).toContain('detail.value = id;');
	});

	test('todos los textos existen en los dos idiomas', () => {
		const sources = ['MicrophoneControl.vue', 'VolumeControl.vue', 'AudioDeviceSelector.vue'].map((file) =>
			read(`src/components/controls/${file}`)
		);
		const keys = new Set(sources.flatMap((text) => [...text.matchAll(/'(components\.\w+\.\w+)'/g)].map((m) => m[1] as string)));
		expect(keys.size).toBeGreaterThan(10);
		for (const locale of ['es', 'en']) {
			const catalog = Bun.YAML.parse(read(`src-tauri/locales/${locale}.yml`)) as Record<string, unknown>;
			for (const key of keys) {
				const value = key.split('.').reduce<unknown>((node, part) => (node as Record<string, unknown>)?.[part], catalog);
				expect(value, `${locale}: ${key}`).toBeString();
			}
		}
	});
});
