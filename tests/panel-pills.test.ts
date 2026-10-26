/**
 * Las píldoras del panel, montadas (vasak-desktop#151): los espacios de
 * trabajo, la distribución de teclado, la red y el volumen.
 *
 * Los componentes de la librería son los publicados; los servicios del
 * escritorio, dobles (`support/panel-doubles.ts`).
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';
import { density, panel } from './support/panel-doubles';
import { WINDOWS_PILL_CAP_REM } from '../src/tools/windows-pill';
import { DEFAULT_PINNED } from '../src/tools/pinned-apps';

const DOUBLES = join(import.meta.dir, 'support', 'panel-doubles.ts');
const dom = useDom();

let Workspaces: any;
let Keyboard: any;
let Network: any;
let Sound: any;
let Battery: any;
let Pinned: any;
let Windows: any;
let Clock: any;

beforeAll(async () => {
	const load = (file: string, name: string) => loadComponent(dom.workdir(), file, DOUBLES, name);
	Workspaces = await load('src/components/panel/WorkspacesPill.vue', 'WorkspacesPill');
	Keyboard = await load('src/components/panel/KeyboardLayoutPill.vue', 'KeyboardLayoutPill');
	Network = await load('src/components/buttons/TrayIconNetwork.vue', 'TrayIconNetwork');
	Sound = await load('src/components/buttons/TrayIconSound.vue', 'TrayIconSound');
	Battery = await load('src/components/buttons/TrayIconBattery.vue', 'TrayIconBattery');
	Pinned = await load('src/components/panel/PinnedAppsPill.vue', 'PinnedAppsPill');
	Windows = await load('src/components/areas/panel/WindowsArea.vue', 'WindowsArea');
	Clock = await load('src/components/widgets/PanelClockWidget.vue', 'PanelClockWidget');
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

	test('si la primera lectura falló, reintenta y muestra el layout', async () => {
		// Wayfire puede no estar listo cuando el panel arranca: la primera
		// lectura devuelve null, pero el reintento a los 2 segundos lo consigue.
		panel.failFirstLayout = true;
		const layout = { short: 'LA', name: 'Spanish (Latin American)', index: 0, count: 1 };
		const view = await render(Keyboard);

		// Al principio no hay pill: la primera lectura falló.
		expect(view.find('[data-panel-pill]').exists()).toBe(false);

		// El reintento se ejecuta después de 2 segundos.
		await new Promise((resolve) => setTimeout(resolve, 2100));
		await settle();

		// Ahora sí: el pill aparece con el layout correcto.
		expect(view.find('[data-panel-pill]').exists()).toBe(true);
		expect(view.find('[data-pill-label]').text()).toBe(layout.short);
		view.unmount();
	});

	test('el widget se apaga desde la configuración', () => {
		// La píldora de teclado depende de `showKeyboard`, como los demás.
		const source = readFileSync(join(ROOT, 'src/views/PanelView.vue'), 'utf8');
		expect(source).toContain('<KeyboardLayoutPill v-if="showKeyboard" />');
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

describe('la barra de ventanas', () => {
	test('no se pliega en ningún ancho: en el panel más angosto siguen todas', async () => {
		// Decisión del usuario (03/10/2026): la barra de ventanas queda siempre.
		for (const level of ['full', 'compact', 'tight'] as const) {
			density.value = level;
			const view = await render(Windows);
			expect(view.find('[data-windows-pill]').exists(), level).toBe(true);
			expect(view.findAll('[data-window]').map((button) => button.text())).toEqual(['Firefox', 'Terminal']);
			view.unmount();
		}
	});

	test('acotada y con scroll adentro, no empuja a las vecinas (vasak-desktop#161)', async () => {
		// Arriba: un ancho máximo la acota y, sin `shrink-0` y con `min-w-0`,
		// cede cuando falta lugar; lo que no entra se desplaza a lo largo de la
		// barra adentro de la píldora.
		const view = await render(Windows);
		const pill = view.find('[data-windows-pill]');

		expect(pill.classes()).toContain('max-w-[20rem]');
		expect(pill.classes()).toContain('overflow-x-auto');
		expect(pill.classes()).toContain('overflow-y-hidden');
		expect(pill.classes()).toContain('min-w-0');
		expect(pill.classes()).not.toContain('shrink-0');
		view.unmount();
	});

	test('en densidad compacta reserva su ancho con `shrink-0` y se topa a tres iconos (vasak-desktop#251)', async () => {
		// En compacto la barra se encoge a su contenido (`w-fit`) y no le reservaba
		// ancho a la píldora, que quedaba en su mínimo aunque sobrara lugar. Con
		// `shrink-0` reserva su ancho. El viejo `max-w-[90vw]` dejaba crecer la
		// píldora casi hasta la pantalla y recién ahí desplazaba; ahora se topa a
		// tres iconos (`:style` sacado de las medidas reales) y desplaza el resto.
		// En distribuida no lleva `shrink-0` (cede a las vecinas, #161).
		panel.panelLayout.value = 'compact';
		const compact = await render(Windows);
		const compactPill = compact.find('[data-windows-pill]');
		expect(compactPill.classes()).toContain('shrink-0');
		expect(compactPill.classes()).toContain('overflow-x-auto');
		// Ni el tope distribuido ni el viejo `max-w-[90vw]`.
		expect(compactPill.classes()).not.toContain('max-w-[20rem]');
		expect(compactPill.classes()).not.toContain('max-w-[90vw]');
		// El tope de tres iconos va por estilo en línea (gana sobre `max-w-full`).
		expect(compactPill.attributes('style')).toContain(`max-width: ${WINDOWS_PILL_CAP_REM}rem`);
		compact.unmount();

		panel.panelLayout.value = 'distributed';
		const distributed = await render(Windows);
		const distributedPill = distributed.find('[data-windows-pill]');
		expect(distributedPill.classes()).not.toContain('shrink-0');
		expect(distributedPill.classes()).toContain('max-w-[20rem]');
		expect(distributedPill.classes()).not.toContain('max-w-[90vw]');
		// En distribuida el tope lo pone la clase, no el estilo en línea.
		expect(distributedPill.attributes('style') ?? '').not.toContain('max-width');
		distributed.unmount();
	});

	test('de costado no reserva ancho aunque sea compacta: apila y desplaza en vertical', async () => {
		// De costado la píldora toma el alto que sobra y desplaza hacia abajo; el
		// `shrink-0` horizontal de compacto no aplica, que ahí el eje es el alto.
		panel.panelLayout.value = 'compact';
		panel.vertical.value = true;
		const view = await render(Windows);
		expect(view.find('[data-windows-pill]').classes()).not.toContain('shrink-0');
		view.unmount();
	});

	test('de costado el scroll es vertical y toma el alto que sobra', async () => {
		panel.vertical.value = true;
		const view = await render(Windows);
		const pill = view.find('[data-windows-pill]');

		expect(pill.classes()).toContain('overflow-y-auto');
		expect(pill.classes()).toContain('overflow-x-hidden');
		expect(pill.classes()).toContain('flex-1');
		expect(pill.classes()).not.toContain('max-w-[20rem]');
		// De costado el tope es de **alto** (tres iconos), no de ancho.
		expect(pill.attributes('style')).toContain(`max-height: ${WINDOWS_PILL_CAP_REM}rem`);
		expect(pill.attributes('style') ?? '').not.toContain('max-width');
		view.unmount();
	});

	test('la barra de desplazamiento va oculta, en las dos formas', () => {
		// En el DOM de prueba no se puede leer el estilo calculado del
		// pseudoelemento; se comprueba que el estilo acotado esté declarado.
		const source = readFileSync(
			join(ROOT, 'src/components/areas/panel/WindowsArea.vue'),
			'utf8'
		);
		expect(source).toContain('scrollbar-width: none');
		expect(source).toContain('::-webkit-scrollbar');
		expect(source).toContain('display: none');
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

describe('apagar textos del panel desde la configuración (vasak-desktop#251)', () => {
	test('el nombre de la red se apaga sin importar la densidad, y no es regresivo', async () => {
		// Con lugar y conectado, el nombre sale (como siempre).
		const on = await render(Network);
		expect(on.find('[data-pill-label]').text()).toBe('Fibernet-IA-5G-Departamento');
		on.unmount();

		// Apagado, no sale aunque haya lugar y haya conexión: el ajuste se combina
		// con la densidad, no la pisa.
		panel.showNetworkName.value = false;
		const off = await render(Network);
		expect(off.find('[data-pill-label]').exists()).toBe(false);
		// Pero la píldora sigue, rellena y conectada: sólo se fue el texto.
		expect(off.find('[data-panel-pill]').attributes('data-active')).toBe('true');
		off.unmount();
	});

	test('el número del volumen se apaga aunque haya lugar', async () => {
		const on = await render(Sound);
		expect(on.find('[data-pill-label]').text()).toBe('50');
		on.unmount();

		panel.showVolumeNumber.value = false;
		const off = await render(Sound);
		expect(off.find('[data-pill-label]').exists()).toBe(false);
		off.unmount();
	});

	test('el número de la batería se apaga, y sigue combinándose con la densidad', async () => {
		const on = await render(Battery);
		expect(on.find('[data-pill-label]').text()).toBe('100');
		on.unmount();

		// Apagado: sin número aunque haya batería y lugar.
		panel.showBatteryNumber.value = false;
		const off = await render(Battery);
		expect(off.find('[data-pill-label]').exists()).toBe(false);
		off.unmount();

		// Encendido pero en el panel más angosto: tampoco, porque el número no entra.
		panel.showBatteryNumber.value = true;
		density.value = 'tight';
		const tight = await render(Battery);
		expect(tight.find('[data-pill-label]').exists()).toBe(false);
		tight.unmount();
	});
});

describe('las aplicaciones ancladas (vasak-desktop#251)', () => {
	test('sin configuración van las dos de siempre, con su nombre traducido', async () => {
		const view = await render(Pinned);
		const buttons = view.findAll('[data-pinned-app]');
		expect(buttons).toHaveLength(DEFAULT_PINNED.length);
		// El `data-pinned-app` es la ruta del `.desktop` (el `id` de la entrada).
		expect(buttons.map((b) => b.attributes('data-pinned-app'))).toEqual(
			DEFAULT_PINNED.map((app) => app.id)
		);
		view.unmount();
	});

	test('una lista vacía deja la zona sin anclados', async () => {
		panel.pinnedApps.value = [];
		const view = await render(Pinned);
		expect(view.findAll('[data-pinned-app]')).toHaveLength(0);
		view.unmount();
	});

	test('tocar una la lanza por `open_app` con la ruta del `.desktop`, no por el shell', async () => {
		const terminal = {
			id: '/usr/share/applications/vasak-terminal.desktop',
			name: 'Terminal',
			icon: 'utilities-terminal',
			exec: 'vasak-terminal',
		};
		panel.pinnedApps.value = [terminal];
		const view = await render(Pinned);
		await view.find('[data-pinned-app]').trigger('click');
		await settle();

		const call = panel.calls.find((entry) => entry.name === 'openApp');
		expect(call).toBeDefined();
		// Pasa `path` = `id` (la ruta del `.desktop`), que es lo que lee `open_app`.
		expect(call?.args[0]).toEqual({ path: terminal.id });
		view.unmount();
	});
});

describe('el reloj', () => {
	test('arriba: la hora alta a un lado y la fecha en dos renglones (vasak-desktop#168)', async () => {
		const view = await render(Clock);

		expect(view.find('[data-clock-time]').text()).toMatch(/^\d{2}:\d{2}$/);
		// La fecha apilada: el día arriba, el número y el mes abajo.
		const dateLines = view.find('[data-clock-date]').findAll('span');
		expect(dateLines).toHaveLength(2);
		expect(dateLines[0]?.text().length).toBeGreaterThan(0);
		expect(dateLines[1]?.text()).toMatch(/\d/);
		view.unmount();
	});

	test('en un panel angosto se pliega la fecha y queda la hora', async () => {
		density.value = 'compact';
		const view = await render(Clock);

		expect(view.find('[data-clock-time]').text()).toMatch(/^\d{2}:\d{2}$/);
		expect(view.find('[data-clock-date]').exists()).toBe(false);
		view.unmount();
	});

	test('de costado la hora va apilada sobre los minutos', async () => {
		panel.vertical.value = true;
		const view = await render(Clock);

		// De costado no hay contenido propio: la hora y el minuto van por
		// `label`/`caption`, que es lo que entra en la columna de 36 píxeles.
		expect(view.find('[data-clock-date]').exists()).toBe(false);
		expect(view.find('[data-pill-label]').text()).toMatch(/^\d{2}$/);
		expect(view.find('[data-pill-caption]').text()).toMatch(/^\d{2}$/);
		view.unmount();
	});

	test('tocarlo abre el tablero de fecha colgado de la píldora', async () => {
		const view = await render(Clock);
		await view.find('[data-clock-pill]').trigger('click');
		await settle();

		const call = panel.calls.find((entry) => entry.name === 'toggleApplet');
		expect(call?.args[0]).toBe('date');
		view.unmount();
	});
});

describe('las píldoras se aplanan cuando hay superficie', () => {
	test('con superficie la píldora va plana: sin su fondo propio (`flat`)', async () => {
		// Flotante, barra, dock y trapecio dibujan una superficie detrás; la isla
		// con fondo de cada píldora encima daba doble fondo. Con `hasSurface` la
		// píldora recibe `flat` y queda transparente (vue-libvasak 2.15.0).
		panel.hasSurface.value = true;
		const view = await render(Clock);
		const pill = view.find('[data-panel-pill]');
		expect(pill.classes()).toContain('bg-transparent');
		expect(pill.classes()).not.toContain('bg-ui-shell');
		view.unmount();
	});

	test('en modo píldoras (sin superficie) conserva su isla con fondo', async () => {
		panel.hasSurface.value = false;
		const view = await render(Clock);
		const pill = view.find('[data-panel-pill]');
		expect(pill.classes()).toContain('bg-ui-shell');
		expect(pill.classes()).not.toContain('bg-transparent');
		view.unmount();
	});

	test('la píldora de los espacios de trabajo se aplana con superficie', async () => {
		// Con flotante, barra, dock o trapecio la grilla de espacios pintaba su
		// isla encima del panel: es la única píldora que no recibía `flat`.
		panel.workspaces.value = { count: 4, active: 1, columns: 2, output_id: 1 };
		panel.hasSurface.value = true;
		const view = await render(Workspaces);
		const pill = view.find('[data-workspace-switcher] [data-panel-pill]');
		expect(pill.exists()).toBe(true);
		expect(pill.classes()).toContain('bg-transparent');
		expect(pill.classes()).not.toContain('bg-ui-shell');
		view.unmount();
	});

	test('en modo píldoras la grilla de espacios conserva su isla con fondo', async () => {
		panel.workspaces.value = { count: 4, active: 1, columns: 2, output_id: 1 };
		panel.hasSurface.value = false;
		const view = await render(Workspaces);
		const pill = view.find('[data-workspace-switcher] [data-panel-pill]');
		expect(pill.exists()).toBe(true);
		expect(pill.classes()).toContain('bg-ui-shell');
		expect(pill.classes()).not.toContain('bg-transparent');
		view.unmount();
	});

	test('la píldora de ventanas también se aplana con superficie', async () => {
		panel.hasSurface.value = true;
		const view = await render(Windows);
		const pill = view.find('[data-windows-pill]');
		expect(pill.classes()).toContain('bg-transparent');
		expect(pill.classes()).not.toContain('bg-ui-shell');
		view.unmount();
	});
});

describe('los gates de PanelView (vasak-desktop#251)', () => {
	// Montar la vista entera arrastra demasiado backend; se comprueba el cableado
	// en la plantilla, como con la campanita de notificaciones.
	const source = readFileSync(join(ROOT, 'src/views/PanelView.vue'), 'utf8');

	test('la píldora de espacios de trabajo depende de `showWorkspaces`', () => {
		expect(source).toContain('<WorkspacesPill v-if="showWorkspaces" />');
	});

	test('la píldora de ventanas depende de `showWindows` en sus dos montajes', () => {
		expect(source).toContain('<WindowsArea v-if="showWindows && windowsFirst" />');
		expect(source).toContain('<WindowsArea v-if="showWindows && !windowsFirst" />');
	});

	test('toma `showWindows` y `showWorkspaces` de la configuración', () => {
		const config = readFileSync(
			join(ROOT, 'src/tools/composables/usePanelConfig.ts'),
			'utf8'
		);
		expect(config).toContain('section.value.windows !== false');
		expect(config).toContain('section.value.workspaces !== false');
	});
});

describe('la píldora de notificaciones', () => {
	test('va al final del panel, en el grupo del extremo (vasak-desktop#160)', () => {
		// Montar la vista entera arrastra demasiado backend; se comprueba el
		// orden en la plantilla: la campanita vive en el grupo del final
		// (`data-panel-end`), no en el del principio.
		const source = readFileSync(join(ROOT, 'src/views/PanelView.vue'), 'utf8');
		const start = source.indexOf('data-panel-start');
		const center = source.indexOf('data-panel-center');
		const end = source.indexOf('data-panel-end');
		const bell = source.indexOf('data-notifications-pill');

		expect(start).toBeGreaterThan(-1);
		expect(end).toBeGreaterThan(-1);
		expect(bell).toBeGreaterThan(-1);
		// Después del principio y del centro: está en el grupo del final.
		expect(bell).toBeGreaterThan(end);
		expect(bell).toBeGreaterThan(center);
	});
});
