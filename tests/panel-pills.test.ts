/**
 * Las píldoras del panel, montadas (vasak-desktop#151): los espacios de
 * trabajo, la distribución de teclado, la red y el volumen.
 *
 * Los componentes de la librería son los publicados; los servicios del
 * escritorio, dobles (`support/panel-doubles.ts`).
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { loadComponent, useDom } from './support/mount-sfc';
import { density, panel } from './support/panel-doubles';

const DOUBLES = join(import.meta.dir, 'support', 'panel-doubles.ts');
const dom = useDom();

let Workspaces: any;
let Keyboard: any;
let Network: any;
let Sound: any;

beforeAll(async () => {
	const load = (file: string, name: string) => loadComponent(dom.workdir(), file, DOUBLES, name);
	Workspaces = await load('src/components/panel/WorkspacesPill.vue', 'WorkspacesPill');
	Keyboard = await load('src/components/panel/KeyboardLayoutPill.vue', 'KeyboardLayoutPill');
	Network = await load('src/components/buttons/TrayIconNetwork.vue', 'TrayIconNetwork');
	Sound = await load('src/components/buttons/TrayIconSound.vue', 'TrayIconSound');
}, 60_000);

beforeEach(() => panel.reset());

const settle = async () => {
	for (let i = 0; i < 4; i++) await nextTick();
};

async function render(component: unknown) {
	const view = mount(component as any, { attachTo: document.body });
	await settle();
	return view;
}

describe('los espacios de trabajo', () => {
	test('un número por espacio de la grilla de Wayfire, el actual en el primario', async () => {
		panel.workspaces.value = { count: 6, active: 4, columns: 3, output_id: 1 };
		const view = await render(Workspaces);
		const buttons = view.findAll('[data-workspace]');

		expect(buttons.map((button) => button.text())).toEqual(['1', '2', '3', '4', '5', '6']);
		expect(buttons[4]?.attributes('aria-current')).toBe('true');
		expect(buttons[4]?.find('[data-workspace-mark]').classes()).toContain('bg-primary');
		view.unmount();
	});

	test('tocar un número pasa a ese espacio, y se marca en el acto', async () => {
		const view = await render(Workspaces);
		await view.findAll('[data-workspace]')[2]?.trigger('click');
		await settle();

		expect(panel.calls).toContainEqual({ name: 'switchWorkspace', args: [2] });
		expect(view.findAll('[data-workspace]')[2]?.attributes('aria-current')).toBe('true');
		view.unmount();
	});

	test('si Wayfire no cambia, la marca vuelve a donde estaba', async () => {
		panel.failSwitch = true;
		const view = await render(Workspaces);
		await view.findAll('[data-workspace]')[3]?.trigger('click');
		await settle();

		expect(view.findAll('[data-workspace]')[0]?.attributes('aria-current')).toBe('true');
		view.unmount();
	});

	test('el aviso de Wayfire mueve la marca y cambia la cantidad', async () => {
		const view = await render(Workspaces);
		panel.emit('workspaces-changed', { count: 4, active: 3, columns: 2, output_id: 2 });
		await settle();

		expect(view.findAll('[data-workspace]')).toHaveLength(4);
		expect(view.findAll('[data-workspace]')[3]?.attributes('aria-current')).toBe('true');
		view.unmount();
	});

	test('con un solo espacio, o sin Wayfire, no hay píldora', async () => {
		panel.workspaces.value = { count: 1, active: 0, columns: 1, output_id: 1 };
		const one = await render(Workspaces);
		expect(one.find('[data-panel-pill]').exists()).toBe(false);
		one.unmount();

		panel.workspaces.value = null;
		const none = await render(Workspaces);
		expect(none.find('[data-panel-pill]').exists()).toBe(false);
		none.unmount();
	});
});

describe('la distribución de teclado', () => {
	test('muestra el código corto, con un globo que lo explica', async () => {
		// El catálogo del doble está vacío: el texto del globo es la clave, y
		// lo que se fija es que esté.
		const view = await render(Keyboard);
		const pill = view.find('[data-panel-pill]');

		expect(view.find('[data-pill-label]').text()).toBe('LA');
		expect(pill.attributes('title')).toBeTruthy();
		view.unmount();
	});

	test('con una sola configurada informa: no es un botón', async () => {
		const view = await render(Keyboard);
		expect(view.find('[data-panel-pill]').element.tagName).toBe('DIV');
		view.unmount();
	});

	test('con varias es un botón que pasa a la siguiente', async () => {
		panel.layout.value = { short: 'LA', name: 'Spanish (Latin American)', index: 0, count: 2 };
		const view = await render(Keyboard);
		const pill = view.find('[data-panel-pill]');

		expect(pill.element.tagName).toBe('BUTTON');
		await pill.trigger('click');
		await settle();
		expect(panel.calls.map((call) => call.name)).toContain('nextKeyboardLayout');
		expect(view.find('[data-pill-label]').text()).toBe('US');
		view.unmount();
	});
});

describe('la red', () => {
	test('conectada va rellena en el primario con el nombre de la red', async () => {
		const view = await render(Network);
		const pill = view.find('[data-panel-pill]');

		expect(pill.attributes('data-active')).toBe('true');
		expect(pill.classes()).toContain('bg-primary');
		expect(view.find('[data-pill-label]').text()).toBe('Fibernet-IA-5G-Departamento');
		expect(view.find('[data-pill-label]').classes()).toContain('truncate');
		view.unmount();
	});

	test('desconectada es translúcida, redonda y sin nombre', async () => {
		panel.network.value = { ...panel.network.value, is_connected: false };
		const view = await render(Network);
		const pill = view.find('[data-panel-pill]');

		expect(pill.attributes('data-active')).toBeUndefined();
		expect(pill.classes()).toContain('bg-ui-shell');
		expect(view.find('[data-pill-label]').exists()).toBe(false);
		view.unmount();
	});

	test('tocarla abre la vista de la red colgada de la píldora', async () => {
		const view = await render(Network);
		await view.find('[data-panel-pill]').trigger('click');
		await settle();

		const call = panel.calls.find((entry) => entry.name === 'toggleApplet');
		expect(call?.args[0]).toBe('network');
		expect((call?.args[1] as { $el?: Element })?.$el).toBe(view.find('[data-panel-pill]').element);
		view.unmount();
	});

	test('de costado queda el icono solo', async () => {
		panel.vertical.value = true;
		const view = await render(Network);
		expect(view.find('[data-pill-label]').exists()).toBe(false);
		view.unmount();
	});
});

describe('en un panel angosto', () => {
	test('la red queda en el icono, con el nombre en el globo', async () => {
		density.value = 'compact';
		const view = await render(Network);
		expect(view.find('[data-pill-label]').exists()).toBe(false);
		expect(view.find('[data-panel-pill]').attributes('title')).toBeTruthy();
		expect(view.find('[data-panel-pill]').attributes('data-active')).toBe('true');
		view.unmount();
	});

	test('el número del volumen sigue mientras entra, y se pliega en el más angosto', async () => {
		density.value = 'compact';
		const compact = await render(Sound);
		expect(compact.find('[data-pill-label]').text()).toBe('50');
		compact.unmount();

		density.value = 'tight';
		const tight = await render(Sound);
		expect(tight.find('[data-pill-label]').exists()).toBe(false);
		tight.unmount();
	});
});

describe('el volumen', () => {
	test('el icono y el número', async () => {
		const view = await render(Sound);
		expect(view.find('[data-pill-label]').text()).toBe('50');
		expect(view.find('[data-panel-pill]').element.tagName).toBe('BUTTON');
		view.unmount();
	});

	test('en silencio no hay número', async () => {
		panel.volume.value = { current: 50, min: 0, max: 100, is_muted: true };
		const view = await render(Sound);
		expect(view.find('[data-pill-label]').exists()).toBe(false);
		view.unmount();
	});
});
