/**
 * La densidad del panel se mide por el VIEWPORT, nunca por la propia barra
 * (vasak-connect… no: vasak-desktop, el crash de compacto).
 *
 * El bug: en layout compacto la `<nav>` es `w-fit`, así que su ancho es el del
 * contenido. Medirla con `ResizeObserver` realimentaba un bucle —medir → plegar
 * textos → cambia el ancho → vuelve a medir— que saturaba el hilo del webview
 * («ResizeObserver loop…») e impedía el primer `.configure` de la capa
 * («Timed out waiting for initial .configure»). La barra NUNCA debe ser lo que
 * se observa; se observa `documentElement`, cuyo tamaño no depende del contenido.
 */
import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { mount } from '@vue/test-utils';
import { defineComponent, h, nextTick, ref } from 'vue';
import { watchPanelDensity } from '../src/tools/composables/usePanelDensity';
import { panelDensityForTests } from '../src/tools/composables/usePanelDensity';
import { COMPACT_FROM, FULL_FROM } from '../src/tools/panel-density';

/** Un `ResizeObserver` doble que recuerda a quién observa y deja disparar. */
class FakeResizeObserver {
	static observed: Element[] = [];
	static callbacks: ResizeObserverCallback[] = [];
	constructor(cb: ResizeObserverCallback) {
		FakeResizeObserver.callbacks.push(cb);
	}
	observe(target: Element) {
		FakeResizeObserver.observed.push(target);
	}
	disconnect() {}
	unobserve() {}
	static fire() {
		for (const cb of FakeResizeObserver.callbacks) cb([], this as unknown as ResizeObserver);
	}
	static reset() {
		FakeResizeObserver.observed = [];
		FakeResizeObserver.callbacks = [];
	}
}

/** Fija el ancho que informa el viewport (`documentElement`). */
function setViewportWidth(width: number) {
	document.documentElement.getBoundingClientRect = (() => ({
		width,
		height: 38,
		top: 0,
		left: 0,
		right: width,
		bottom: 38,
		x: 0,
		y: 0,
		toJSON: () => ({}),
	})) as Element['getBoundingClientRect'];
}

const mountWatcher = () =>
	mount(
		defineComponent({
			setup() {
				watchPanelDensity(ref(false));
				return () => h('div');
			},
		})
	);

beforeEach(() => {
	FakeResizeObserver.reset();
	(globalThis as { ResizeObserver: unknown }).ResizeObserver = FakeResizeObserver;
});

afterEach(() => {
	FakeResizeObserver.reset();
});

describe('watchPanelDensity', () => {
	test('observa el viewport (documentElement), no una barra que se mide a sí misma', () => {
		setViewportWidth(FULL_FROM);
		const wrapper = mountWatcher();
		expect(FakeResizeObserver.observed).toContain(document.documentElement);
		wrapper.unmount();
	});

	test('la densidad sale del ancho del viewport', async () => {
		setViewportWidth(FULL_FROM + 10);
		const wrapper = mountWatcher();
		await nextTick();
		expect(panelDensityForTests.value).toBe('full');
		wrapper.unmount();
	});

	test('un viewport angosto pliega aunque el contenido crezca (sin realimentación)', async () => {
		// Clave del arreglo: el tamaño del contenido de la barra es irrelevante;
		// sólo cuenta el viewport, así que no hay bucle medir→plegar→medir.
		setViewportWidth(COMPACT_FROM - 1);
		const wrapper = mountWatcher();
		FakeResizeObserver.fire();
		await nextTick();
		expect(panelDensityForTests.value).toBe('tight');
		wrapper.unmount();
	});
});
