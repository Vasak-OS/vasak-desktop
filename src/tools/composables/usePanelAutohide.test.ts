import { beforeAll, describe, expect, test } from 'bun:test';
import { mount } from '@vue/test-utils';
import { defineComponent, h, nextTick, ref } from 'vue';
import { useDom } from '../../../tests/support/mount-sfc';
import { HIDE_DELAY_MS, type PanelAutohide, usePanelAutohide } from './usePanelAutohide';

useDom();

// La ventana del panel es la franja: angosta de alto. Con el panel arriba, el
// borde interior —por donde sale el puntero— es el de abajo (y grande).
const VIEWPORT_H = 38;
beforeAll(() => {
	Object.defineProperty(window, 'innerWidth', { value: 1920, configurable: true });
	Object.defineProperty(window, 'innerHeight', { value: VIEWPORT_H, configurable: true });
});

function mountAutohide(
	autohide: ReturnType<typeof ref<boolean>>,
	appletOpen: ReturnType<typeof ref<boolean>>
) {
	let api!: PanelAutohide;
	const wrapper = mount(
		defineComponent({
			setup() {
				api = usePanelAutohide(autohide as never, ref('top') as never, appletOpen as never);
				return () => h('nav');
			},
		})
	);
	return { api, wrapper };
}

const moveTo = (y: number) =>
	document.dispatchEvent(new MouseEvent('pointermove', { clientX: 960, clientY: y }));
const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const afterHide = () => wait(HIDE_DELAY_MS + 120);

describe('usePanelAutohide', () => {
	test('prender el auto-ocultar esconde la barra; apagarlo la devuelve', async () => {
		const autohide = ref(false);
		const applet = ref(false);
		const { api, wrapper } = mountAutohide(autohide, applet);
		await nextTick();
		expect(api.hidden.value).toBe(false);

		autohide.value = true;
		await nextTick();
		expect(api.hidden.value).toBe(true);

		autohide.value = false;
		await nextTick();
		expect(api.hidden.value).toBe(false);
		wrapper.unmount();
	});

	test('el puntero contra el borde exterior revela; apuntando en el medio, se queda', async () => {
		const autohide = ref(true);
		const applet = ref(false);
		const { api, wrapper } = mountAutohide(autohide, applet);
		await nextTick();
		expect(api.hidden.value).toBe(true);

		moveTo(2); // borde exterior (la línea de revelado)
		expect(api.hidden.value).toBe(false);

		moveTo(18); // en el medio, apuntando a algo
		await afterHide();
		expect(api.hidden.value).toBe(false); // no se esconde: no está saliendo
		wrapper.unmount();
	});

	test('al llegar el puntero al borde interior (saliendo), se esconde', async () => {
		const autohide = ref(true);
		const applet = ref(false);
		const { api, wrapper } = mountAutohide(autohide, applet);
		await nextTick();

		moveTo(18); // revelada, en el medio
		expect(api.hidden.value).toBe(false);

		moveTo(VIEWPORT_H - 1); // contra el borde interior: saliendo
		expect(api.hidden.value).toBe(false); // todavía no: hay una espera
		await afterHide();
		expect(api.hidden.value).toBe(true);
		wrapper.unmount();
	});

	test('con un applet abierto la barra se queda a la vista', async () => {
		const autohide = ref(true);
		const applet = ref(false);
		const { api, wrapper } = mountAutohide(autohide, applet);
		await nextTick();

		applet.value = true;
		await nextTick();
		expect(api.hidden.value).toBe(false);

		// Ni llegando al borde interior se esconde mientras el applet siga abierto.
		moveTo(VIEWPORT_H - 1);
		await afterHide();
		expect(api.hidden.value).toBe(false);

		// Al cerrarse el applet, se esconde tras la espera.
		applet.value = false;
		await afterHide();
		expect(api.hidden.value).toBe(true);
		wrapper.unmount();
	});
});
