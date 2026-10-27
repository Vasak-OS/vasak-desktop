/**
 * El cross-slide del centro de control: notificaciones (A) y ajustes (B) son
 * dos paneles lado a lado en un carril que se traslada un panel entero.
 *
 * Se prueba la estructura y la lógica sobre la vista montada con sus
 * componentes doblados: qué panel se ve, cuál es `inert`, que los dos siguen
 * montados y que la música y el volumen no viven en el carril. Lo que NO se
 * prueba acá es el movimiento: la duración y el pintado de la transición no
 * los mide happy-dom; se verificó midiendo el DOM en el navegador (el carril
 * se corre exactamente un panel en cada sentido y A y B calzan con el
 * escenario).
 */
import { afterEach, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { mount, type VueWrapper } from '@vue/test-utils';
import { nextTick } from 'vue';
import { lifecycle, sharedEvents } from './support/cross-slide-doubles';
import { loadControlCenterView } from './support/load-control-center-view';
import { useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'cross-slide-doubles.ts');
const dom = useDom();

const settle = async () => {
	for (let i = 0; i < 8; i++) {
		await nextTick();
		await Promise.resolve();
	}
};

describe('el cross-slide del centro', () => {
	let View: any;
	let center: VueWrapper;
	beforeAll(async () => {
		View = await loadControlCenterView(dom.workdir(), DOUBLES);
	}, 60_000);
	beforeEach(async () => {
		lifecycle.notificationAreaMounts = 0;
		center = mount(View, { attachTo: document.body });
		await settle();
	});

	afterEach(() => center.unmount());

	const track = () => center.find('[data-track]');
	const panel = (name: 'notifications' | 'settings') => center.find(`[data-panel="${name}"]`);
	const isInert = (name: 'notifications' | 'settings') => panel(name).attributes('inert') !== undefined;
	const toSettings = async () => {
		await center.find('[data-more]').trigger('click');
		await settle();
	};
	const toNotifications = async () => {
		await center.find('[data-notification-summary]').trigger('click');
		await settle();
	};

	test('el carril se traslada un panel entero a los ajustes y vuelve a las notificaciones', async () => {
		expect(center.find('main').attributes('data-mode')).toBe('notifications');
		expect(track().classes()).toContain('translate-x-0');
		expect(track().classes()).not.toContain('-translate-x-full');

		await toSettings();
		expect(center.find('main').attributes('data-mode')).toBe('settings');
		expect(track().classes()).toContain('-translate-x-full');
		expect(track().classes()).not.toContain('translate-x-0');

		await toNotifications();
		expect(center.find('main').attributes('data-mode')).toBe('notifications');
		expect(track().classes()).toContain('translate-x-0');
		expect(track().classes()).not.toContain('-translate-x-full');
	});

	test('el carril anima con movimiento y cambia de golpe con movimiento reducido', () => {
		// Un solo lugar donde se apaga: la transición, no el transform.
		expect(track().classes()).toEqual(
			expect.arrayContaining(['transition-transform', 'duration-200', 'motion-reduce:transition-none'])
		);
	});

	test('los dos paneles son hermanos dentro del carril y miden el ancho entero', () => {
		const panels = center.findAll('[data-panel]');
		expect(panels.map((p) => p.attributes('data-panel'))).toEqual(['notifications', 'settings']);
		for (const p of panels) {
			expect(p.element.parentElement === track().element).toBe(true);
			expect(p.classes()).toEqual(expect.arrayContaining(['w-full', 'shrink-0']));
		}
		// El carril está dentro del escenario que recorta lo que se sale.
		expect(center.find('[data-stage]').element.contains(track().element)).toBe(true);
		expect(center.find('[data-stage]').classes()).toContain('overflow-hidden');
	});

	test('los dos paneles siguen montados al pasar de uno a otro, con su contenido', async () => {
		for (const step of [undefined, toSettings, toNotifications]) {
			await step?.();
			expect(panel('notifications').exists()).toBe(true);
			expect(panel('settings').exists()).toBe(true);
			expect(panel('notifications').find('[data-notifications]').exists()).toBe(true);
			expect(panel('notifications').find('[data-quick-toggles]').exists()).toBe(true);
			expect(panel('notifications').find('[data-primary-brightness-box]').exists()).toBe(true);
			expect(panel('settings').find('[data-notification-summary]').exists()).toBe(true);
		}
	});

	test('la lista de notificaciones no se desmonta al pasar a ajustes: es la misma instancia', async () => {
		const list = center.find('[data-fake-notifications]').element;
		expect(lifecycle.notificationAreaMounts).toBe(1);

		await toSettings();
		expect(list.isConnected).toBe(true);
		expect(center.find('[data-fake-notifications]').element === list).toBe(true);
		expect(panel('notifications').element.contains(list)).toBe(true);

		await toNotifications();
		expect(center.find('[data-fake-notifications]').element === list).toBe(true);
		// Una sola vez montada en todo el recorrido: nada se volvió a pedir.
		expect(lifecycle.notificationAreaMounts).toBe(1);
	});

	test('la lista de notificaciones conserva su alto mínimo y su desplazamiento propio', () => {
		const list = center.find('[data-notifications]');
		expect(list.classes()).toEqual(expect.arrayContaining(['min-h-40', 'flex-1']));
		expect(list.element.closest('[data-scroll]')?.classList.contains('overflow-y-auto')).toBe(true);
	});

	test('el panel que no se ve queda inert y el que se ve no', async () => {
		expect(isInert('notifications')).toBe(false);
		expect(isInert('settings')).toBe(true);

		await toSettings();
		expect(isInert('notifications')).toBe(true);
		expect(isInert('settings')).toBe(false);

		await toNotifications();
		expect(isInert('notifications')).toBe(false);
		expect(isInert('settings')).toBe(true);
	});

	test('con el calendario abierto el carril entero queda inert y la ficha va encima', async () => {
		expect(track().attributes('inert')).toBeUndefined();
		await center.find('[data-user-date]').trigger('click');
		await settle();

		const sheet = center.find('[data-calendar-sheet]');
		expect(sheet.exists()).toBe(true);
		expect(track().attributes('inert')).toBeDefined();
		expect(sheet.classes()).toEqual(expect.arrayContaining(['absolute', 'inset-0', 'z-10']));
		// La ficha no es parte del carril: lo tapa, no se desliza con él.
		expect(track().element.contains(sheet.element)).toBe(false);
		expect(center.find('[data-stage]').element.contains(sheet.element)).toBe(true);

		await center.find('[data-calendar-back]').trigger('click');
		await settle();
		expect(track().attributes('inert')).toBeUndefined();
	});

	test('la música y el volumen están en la barra compartida, fuera del carril, en los dos modos', async () => {
		for (const step of [undefined, toSettings, toNotifications]) {
			await step?.();
			const music = center.find('[data-music-box]').element;
			const volume = center.find('[data-stub-VolumeControl]').element;
			expect(music.isConnected).toBe(true);
			expect(volume.isConnected).toBe(true);
			expect(track().element.contains(music)).toBe(false);
			expect(track().element.contains(volume)).toBe(false);
			expect(center.find('[data-stage]').element.contains(volume)).toBe(false);
			// Ni los esconde un panel `inert`: la barra siempre es operable.
			expect(music.closest('[inert]')).toBeNull();
			expect(volume.closest('[inert]')).toBeNull();
		}
	});

	test('la caja de la música se esconde sin reproductor y se muestra con uno, en cualquier modo', async () => {
		const box = () => center.find('[data-music-box]').element as HTMLElement;
		// Sigue montada aunque no se vea: el widget es quien avisa que hay algo sonando.
		expect(box().style.display).toBe('none');
		expect(center.find('[data-fake-music]').exists()).toBe(true);

		await toSettings();
		await center.find('[data-fake-music]').trigger('click');
		await settle();
		expect(box().style.display).not.toBe('none');

		await toNotifications();
		expect(box().style.display).not.toBe('none');
	});

	test('los mosaicos, el micrófono y la energía están en el panel de ajustes y no existen hasta ver B con el centro abierto', async () => {
		const mounted = () => ({
			tiles: center.find('[data-stub-QuickSettingsPanel]').exists(),
			microphone: center.find('[data-stub-MicrophoneControl]').exists(),
			power: center.find('[data-display-power]').exists(),
		});
		// Sin que el centro se haya mostrado, ni ver B los monta.
		await toSettings();
		expect(mounted()).toEqual({ tiles: false, microphone: false, power: false });
		await toNotifications();

		sharedEvents.get('window-shown')?.();
		await settle();
		// Visible pero todavía en A: tampoco.
		expect(mounted()).toEqual({ tiles: false, microphone: false, power: false });

		await toSettings();
		expect(mounted()).toEqual({ tiles: true, microphone: true, power: true });
		for (const selector of ['[data-stub-QuickSettingsPanel]', '[data-stub-MicrophoneControl]', '[data-display-power]']) {
			expect(panel('settings').find(selector).exists()).toBe(true);
			expect(panel('notifications').find(selector).exists()).toBe(false);
		}

		// Una vez montados no se desmontan al volver a A.
		await toNotifications();
		expect(mounted()).toEqual({ tiles: true, microphone: true, power: true });
	});
});
