/**
 * La estructura plegable del centro de control (vasak-desktop#175, ancla #174).
 *
 * Estado A: las notificaciones con todo el alto que sobra y un mínimo, la fila
 * de interruptores con «más». Estado B: las notificaciones en una línea que las
 * reabre y los mosaicos de ajustes, con la ficha de un mosaico dentro del
 * bloque (lista → ficha con volver). Lo puro se prueba directo; el bloque de
 * mosaicos y la lista de notificaciones, montados con dobles; la vista, por su
 * texto, porque montarla entera arrastra a medio escritorio.
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import {
	modeOnCountChange,
	modeOnOpen,
	summaryKey,
	summaryText,
} from '../src/tools/control-center-mode';
import { availableTiles, CONTROL_CENTER_TILES } from '../src/tools/control-center-tiles';
import { center } from './support/control-center-doubles';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'control-center-doubles.ts');
const dom = useDom();
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

function flatten(value: unknown, prefix = '', out: Record<string, string> = {}) {
	if (value && typeof value === 'object') {
		for (const [key, child] of Object.entries(value)) flatten(child, prefix ? `${prefix}.${key}` : key, out);
	} else if (prefix) out[prefix] = String(value);
	return out;
}
const catalog = (locale: string) =>
	flatten(Bun.YAML.parse(read(`src-tauri/locales/${locale}.yml`)));
const translator = (locale: string) => {
	const strings = catalog(locale);
	return (key: string) => strings[key] ?? key;
};

const settle = async () => {
	for (let i = 0; i < 6; i++) await nextTick();
};

describe('con qué estado abre', () => {
	test('con notificaciones abre en ellas (A); sin ninguna, en los ajustes (B)', () => {
		expect(modeOnOpen(3)).toBe('notifications');
		expect(modeOnOpen(1)).toBe('notifications');
		expect(modeOnOpen(0)).toBe('settings');
	});

	test('si se va la última con el centro abierto, pasa a los ajustes', () => {
		expect(modeOnCountChange(3, 0, 'notifications')).toBe('settings');
		expect(modeOnCountChange(1, 0, 'notifications')).toBe('settings');
	});

	test('una notificación nueva no saca a nadie de los ajustes', () => {
		expect(modeOnCountChange(0, 1, 'settings')).toBe('settings');
		expect(modeOnCountChange(2, 3, 'settings')).toBe('settings');
		expect(modeOnCountChange(3, 2, 'notifications')).toBe('notifications');
	});
});

describe('la línea resumen cuenta bien', () => {
	test('elige la oración entera por cantidad', () => {
		expect(summaryKey(0, 0)).toBe('views.controlCenter.summaryNone');
		expect(summaryKey(1, 1)).toBe('views.controlCenter.summaryOneOne');
		expect(summaryKey(4, 1)).toBe('views.controlCenter.summaryOtherOne');
		expect(summaryKey(3, 2)).toBe('views.controlCenter.summaryOtherOther');
	});

	test('en castellano', () => {
		const t = translator('es');
		expect(summaryText(t, 3, 2)).toBe('3 notificaciones de 2 apps');
		expect(summaryText(t, 4, 1)).toBe('4 notificaciones de 1 app');
		expect(summaryText(t, 1, 1)).toBe('1 notificación de 1 app');
		expect(summaryText(t, 0, 0)).toBe('Sin notificaciones');
	});

	test('en inglés', () => {
		const t = translator('en');
		expect(summaryText(t, 3, 2)).toBe('3 notifications from 2 apps');
		expect(summaryText(t, 1, 1)).toBe('1 notification from 1 app');
		expect(summaryText(t, 0, 0)).toBe('No notifications');
	});
});

describe('los mosaicos', () => {
	test('van los controles que ya existen, en orden', () => {
		expect(CONTROL_CENTER_TILES.map((tile) => tile.id)).toEqual([
			'network',
			'bluetooth',
			'do-not-disturb',
			'night-light',
			'airplane-mode',
			'keep-awake',
			'game-mode',
			'theme',
			'screen-time',
			'search',
		]);
	});

	test('Wi-Fi, Bluetooth y la luz nocturna abren su ficha; la de red sin su botón de cerrar', () => {
		const withDetail = CONTROL_CENTER_TILES.filter((tile) => tile.detail).map((tile) => tile.id);
		expect(withDetail).toEqual(['network', 'bluetooth', 'night-light']);
		expect(CONTROL_CENTER_TILES[0]?.detailProps).toEqual({ hideX: true });
	});

	test('sin adaptador de Bluetooth no hay mosaico de Bluetooth', () => {
		expect(availableTiles(CONTROL_CENTER_TILES, { bluetooth: false }).map((t) => t.id)).not.toContain('bluetooth');
		expect(availableTiles(CONTROL_CENTER_TILES, { bluetooth: true }).map((t) => t.id)).toContain('bluetooth');
	});

	test('se cargan bajo demanda: ningún mosaico entra en el paquete de la vista', () => {
		const registry = read('src/tools/control-center-tiles.ts');
		expect(registry).not.toMatch(/^import \w+ from '@\/components/m);
		expect(registry.match(/defineAsyncComponent\(/g)?.length).toBe(13);
	});

	test('todos los textos de los mosaicos existen en los dos idiomas', () => {
		const sources = [
			'NetworkTile.vue',
			'BluetoothTile.vue',
			'DoNotDisturbTile.vue',
			'NightLightTile.vue',
			'KeepAwakeTile.vue',
			'GameModeTile.vue',
			'AirplaneModeTile.vue',
			'ThemeTile.vue',
			'ScreenTimeTile.vue',
			'SearchTile.vue',
		].map((file) => read(`src/components/controls/tiles/${file}`));
		const keys = new Set(sources.flatMap((text) => [...text.matchAll(/'(components\.ControlCenterTiles\.\w+)'/g)].map((m) => m[1] as string)));
		expect(keys.size).toBeGreaterThan(8);
		for (const locale of ['es', 'en']) {
			const strings = catalog(locale);
			for (const key of keys) expect(strings[key], `${locale}: ${key}`).toBeString();
		}
	});
});

describe('el bloque de mosaicos, montado', () => {
	let Panel: any;
	beforeAll(async () => {
		Panel = await loadComponent(dom.workdir(), 'src/components/areas/control-center/QuickSettingsPanel.vue', DOUBLES, 'QuickSettingsPanel');
	}, 60_000);

	test('muestra un mosaico por ajuste, sin Bluetooth si el equipo no tiene', async () => {
		const view = mount(Panel, { props: { bluetooth: false }, attachTo: document.body });
		// Grupo nativo y con nombre, no `role="group"` sobre un div.
		const group = view.find('[data-tile-grid]');
		expect(group.element.tagName).toBe('FIELDSET');
		expect(group.attributes('role')).toBeUndefined();
		expect(group.attributes('aria-label')).toBeTruthy();
		await settle();
		expect(view.findAll('[data-fake-tile]').map((tile) => tile.text())).toEqual(['network', 'theme', 'screen-time', 'search']);
		view.unmount();
	});

	test('dos columnas por consulta de contenedor, una si el bloque es angosto', async () => {
		const view = mount(Panel, { props: { bluetooth: true }, attachTo: document.body });
		await settle();
		const grid = view.find('[data-tile-grid]');
		expect(grid.classes()).toContain('grid-cols-1');
		expect(grid.classes()).toContain('@[17rem]:grid-cols-2');
		expect(view.find('[data-quick-settings]').classes()).toContain('@container');
		view.unmount();
	});

	test('lista → ficha con volver, dentro del bloque, y el foco acompaña', async () => {
		const view = mount(Panel, { props: { bluetooth: true }, attachTo: document.body });
		await settle();
		await view.find('[data-fake-tile="network"]').trigger('click');
		await settle();

		expect(view.emitted('update:detail')?.at(-1)).toEqual(['network']);
		await view.setProps({ detail: 'network' });
		await settle();
		const detail = view.find('[data-fake-detail="network"]');
		expect(detail.exists()).toBe(true);
		expect(JSON.parse(detail.attributes('data-attrs') ?? '{}')).toEqual({ hideX: true });
		// La lista queda montada pero escondida: no vuelve a pedir nada al volver.
		expect((view.find('[data-tile-grid]').element as HTMLElement).style.display).toBe('none');
		const back = view.find('[data-tile-back]');
		expect(back.exists()).toBe(true);

		await back.trigger('click');
		await settle();
		expect(view.emitted('update:detail')?.at(-1)).toEqual([null]);
		await view.setProps({ detail: null });
		await settle();
		expect(view.find('[data-fake-detail]').exists()).toBe(false);
		// El foco vuelve a la flecha que abrió la ficha.
		expect(document.activeElement).toBe(view.find('[data-fake-tile="network"]').element);
		view.unmount();
	});

	test('la flecha del micrófono abre su ficha en el bloque, y volver le devuelve el foco', async () => {
		// La flecha vive afuera del bloque, abajo, como en la vista.
		const opener = document.createElement('button');
		opener.dataset.sheetOpener = 'audio-input';
		document.body.append(opener);
		const view = mount(Panel, { props: { bluetooth: true, detail: 'audio-input' }, attachTo: document.body });
		await settle();

		const sheet = view.find('[data-fake-detail="audio-input"]');
		expect(sheet.exists()).toBe(true);
		expect(JSON.parse(sheet.attributes('data-attrs') ?? '{}')).toEqual({ kind: 'input' });
		expect((view.find('[data-tile-grid]').element as HTMLElement).style.display).toBe('none');
		// Abierta desde afuera, el foco igual va a «volver».
		expect(document.activeElement).toBe(view.find('[data-tile-back]').element);

		await view.find('[data-tile-back]').trigger('click');
		await settle();
		expect(view.emitted('update:detail')?.at(-1)).toEqual([null]);
		expect(document.activeElement).toBe(opener);
		view.unmount();
		opener.remove();
	});

	test('la de la salida lleva su clase, para elegir entre las salidas', async () => {
		const view = mount(Panel, { props: { bluetooth: true, detail: 'audio-output' }, attachTo: document.body });
		await settle();
		const sheet = view.find('[data-fake-detail="audio-output"]');
		expect(JSON.parse(sheet.attributes('data-attrs') ?? '{}')).toEqual({ kind: 'output' });
		view.unmount();
	});

	test('de la ficha de salida a la de entrada sin volver: lista entradas, no salidas', async () => {
		const view = mount(Panel, { props: { bluetooth: true, detail: 'audio-output' }, attachTo: document.body });
		await settle();
		expect(view.find('[data-fake-detail="audio-output"]').exists()).toBe(true);

		// La flecha del micrófono con la ficha de salida abierta.
		await view.setProps({ detail: 'audio-input' });
		await settle();
		expect(view.find('[data-fake-detail="audio-output"]').exists()).toBe(false);
		expect(view.find('[data-fake-detail="audio-input"]').exists()).toBe(true);
		view.unmount();
	});

	test('un mosaico sin ficha no abre nada en el bloque', async () => {
		const view = mount(Panel, { props: { bluetooth: true, detail: 'theme' }, attachTo: document.body });
		await settle();
		expect(view.find('[data-tile-back]').exists()).toBe(false);
		expect(view.find('[data-tile-grid]').isVisible()).toBe(true);
		view.unmount();
	});
});

describe('la lista de notificaciones avisa cuántas hay', () => {
	let Area: any;
	beforeAll(async () => {
		Area = await loadComponent(dom.workdir(), 'src/components/areas/control-center/NotificationArea.vue', DOUBLES, 'NotificationArea');
	}, 60_000);
	beforeEach(() => center.reset());

	const notification = (id: number, app: string) => ({
		id,
		app_name: app,
		app_icon: '',
		summary: `n${id}`,
		body: '',
		timestamp: 0,
		seen: false,
	});

	test('con la primera carga, y otra vez cuando cambian', async () => {
		center.notifications.value = [notification(1, 'Telegram'), notification(2, 'Telegram'), notification(3, 'Firefox')];
		const view = mount(Area, { attachTo: document.body });
		await settle();
		// Antes de cargar avisa cero, pero dice que todavía no es la cuenta de verdad.
		expect(view.emitted('summary')?.[0]).toEqual([{ count: 0, apps: 0, loaded: false }]);
		expect(view.emitted('summary')?.at(-1)).toEqual([{ count: 3, apps: 2, loaded: true }]);

		center.handlers.get('notification-delta')?.({ items: [] });
		await settle();
		expect(view.emitted('summary')?.at(-1)).toEqual([{ count: 0, apps: 0, loaded: true }]);
		view.unmount();
	});
});

describe('la vista', () => {
	const VIEW = read('src/views/ControlCenterView.vue');

	test('A: la lista con todo el alto que sobra y un mínimo de unas dos tarjetas', () => {
		expect(VIEW).toMatch(/<NotificationArea\s+v-show="!showsSettings && !calendarOpen"\s+class="min-h-40 flex-1"/);
		// La lista se desplaza sola: con muchas notificaciones el mínimo se
		// respeta y lo de abajo no se va de la pantalla.
		const area = read('src/components/areas/control-center/NotificationArea.vue');
		expect(area).toMatch(/class="flex min-h-0 flex-1 flex-col gap-1\.5 overflow-y-auto/);
	});

	test('en una pantalla baja se desplaza el bloque de arriba, nunca se pisan', () => {
		expect(VIEW).toMatch(/class="flex min-h-0 flex-1 flex-col w-full gap-2 overflow-y-auto p-2" data-top/);
	});

	test('B: la línea resumen reabre las notificaciones, que nunca desaparecen', () => {
		expect(VIEW).toMatch(/<ListRow\s+v-if="showsSettings && !calendarOpen"[\s\S]*?data-notification-summary\s+@click="showNotifications"/);
		expect(VIEW).toContain(':title="summaryLabel"');
		expect(VIEW).toContain("mode.value = 'notifications'");
	});

	test('«más» abre B; los interruptores se parten en renglones si no entran', () => {
		expect(VIEW).toMatch(/<ToggleControl\s+name="go-up"[\s\S]*?aria-expanded="false"[\s\S]*?@click="showSettings"/);
		expect(VIEW).toMatch(/class="flex w-full flex-wrap justify-between gap-2"\s+data-quick-toggles/);
	});

	test('los mosaicos no se montan hasta ver B con el centro abierto', () => {
		expect(VIEW).toMatch(/<QuickSettingsPanel\s+v-if="tilesMounted"\s+v-show="showsSettings && !calendarOpen"/);
		expect(VIEW).toContain('if (settings && shown) tilesMounted.value = true;');
		// Empieza escondido: el centro se crea al iniciar la sesión.
		expect(VIEW).toContain('const visible = ref(false);');
	});

	test('cada apertura decide de nuevo y cierra la ficha que hubiera', () => {
		const shown = VIEW.slice(VIEW.indexOf("useSharedEvent('window-shown'"));
		expect(shown).toContain('detail.value = null;');
		expect(shown).toContain('mode.value = modeOnOpen(summary.value.count)');
	});

	test('la sesión va debajo del usuario: la fila de Bloquear y las del diálogo', () => {
		const template = VIEW.slice(VIEW.indexOf('<template>'));
		// Sin `/>`: la tarjeta lleva atributos (el calendario, #183), y un
		// `indexOf` que no encuentra nada da -1 y pasa cualquier comparación.
		const user = template.indexOf('<UserControlCenterCard');
		const row = template.indexOf('<SessionActionsRow');
		const phone = template.indexOf('<PhoneControlCenterCard');
		expect(user).toBeGreaterThan(-1);
		expect(row).toBeGreaterThan(user);
		expect(phone).toBeGreaterThan(row);
		// Lo que hace cada botón se prueba montado en `session-row.test.ts`.
		expect(VIEW).not.toContain('<PowerActions');
	});

	test('no queda nada dibujado a mano: ni colores ni radios', () => {
		const template = VIEW.slice(VIEW.indexOf('<template>'));
		expect(template).not.toMatch(/#[0-9a-f]{3,6}\b|rgb\(|rounded-(?:md|lg|xl)\b|backdrop-blur/);
	});
});

describe('la ficha de red se puede abrir y cerrar sin dejar nada corriendo', () => {
	test('saca su oyente de visibilidad y frena el sondeo al desmontarse', () => {
		const area = read('src/components/areas/network/NetworkControlArea.vue');
		expect(area).toContain("document.addEventListener('visibilitychange', onVisibilityChange)");
		expect(area).toContain("document.removeEventListener('visibilitychange', onVisibilityChange)");
		expect(area).toMatch(/onUnmounted\(\(\) => \{\s+disposed = true;/);
	});
});

describe('la tarjeta del usuario en un centro angosto', () => {
	test('la hora baja debajo del nombre por consulta de contenedor, sin cortarse', () => {
		const card = read('src/components/cards/UserControlCenterCard.vue');
		expect(card).toContain('<div class="@container w-full min-w-0">');
		expect(card).toMatch(/flex-col[^"]*@\[19rem\]:flex-row/);
	});
});
