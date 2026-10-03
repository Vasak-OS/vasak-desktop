import { describe, expect, test } from 'bun:test';
import { mount } from '@vue/test-utils';
import { defineComponent, h, nextTick, ref } from 'vue';
import { tauriCalls, useDom } from '../../tests/support/mount-sfc';
import { usePanelInputRegion } from './composables/usePanelInputRegion';
import { type Measurable, PILL_SELECTOR, pillRects, sameRects } from './panel-input-region';

/** Una caja de mentira: happy-dom no maqueta, así que la medida se da. */
const box = (x: number, y: number, width: number, height: number): Measurable => ({
	getBoundingClientRect: () => ({ x, y, width, height }),
});

describe('la región de entrada del panel', () => {
	test('cada píldora visible es un rectángulo', () => {
		expect(pillRects([box(4, 2, 32, 32), box(40, 2, 180, 32)])).toEqual([
			{ x: 4, y: 2, width: 32, height: 32 },
			{ x: 40, y: 2, width: 180, height: 32 },
		]);
	});

	test('las escondidas no suman: la bandeja vacía no se queda con los clics', () => {
		expect(pillRects([box(0, 0, 0, 0), box(10, 2, 0, 32), box(50, 2, 32, 32)])).toEqual([
			{ x: 50, y: 2, width: 32, height: 32 },
		]);
	});

	test('una píldora adentro de otra ya la cubre la de afuera', () => {
		const outer = box(100, 2, 200, 32);
		const inner = box(110, 2, 32, 32);
		const rects = pillRects([outer, inner], (a, b) => a === outer && b === inner);
		expect(rects).toEqual([{ x: 100, y: 2, width: 200, height: 32 }]);
	});

	test('dos medidas iguales al píxel no repiten el pedido', () => {
		const a = [{ x: 4, y: 2, width: 32, height: 32 }];
		expect(sameRects(a, [{ x: 4.2, y: 2, width: 31.9, height: 32 }])).toBe(true);
		expect(sameRects(a, [{ x: 8, y: 2, width: 32, height: 32 }])).toBe(false);
		expect(sameRects(a, [])).toBe(false);
	});

	test('busca las píldoras por la marca de la librería', () => {
		expect(PILL_SELECTOR).toBe('[data-panel-pill]');
	});
});

describe('el panel le manda sus píldoras al backend', () => {
	useDom();

	test('al montarse pide set_panel_input_region con las píldoras, no con la barra entera', async () => {
		const realFrame = globalThis.requestAnimationFrame;
		globalThis.requestAnimationFrame = ((callback: FrameRequestCallback) => {
			callback(0);
			return 1;
		}) as typeof requestAnimationFrame;
		tauriCalls.length = 0;

		const Bar = defineComponent({
			setup() {
				const bar = ref<HTMLElement | null>(null);
				usePanelInputRegion(bar);
				return () =>
					h('nav', { ref: bar }, [
						h('div', { 'data-panel-pill': '', id: 'search' }),
						h('div', { 'data-panel-pill': '', id: 'clock' }),
						h('div', { id: 'gap' }),
					]);
			},
		});

		const measures: Record<string, [number, number, number, number]> = {
			search: [4, 2, 32, 32],
			clock: [900, 2, 120, 32],
		};
		const original = HTMLElement.prototype.getBoundingClientRect;
		HTMLElement.prototype.getBoundingClientRect = function (this: HTMLElement) {
			const [x, y, width, height] = measures[this.id] ?? [0, 0, 1920, 36];
			return {
				x,
				y,
				width,
				height,
				top: y,
				left: x,
				right: x + width,
				bottom: y + height,
				toJSON: () => ({}),
			} as DOMRect;
		};

		try {
			const view = mount(Bar, { attachTo: document.body });
			await nextTick();
			await nextTick();

			const calls = tauriCalls.filter((call) => call.cmd === 'set_panel_input_region');
			expect(calls.length).toBeGreaterThan(0);
			expect((calls.at(-1)?.args as { rects: unknown } | undefined)?.rects).toEqual([
				{ x: 4, y: 2, width: 32, height: 32 },
				{ x: 900, y: 2, width: 120, height: 32 },
			]);
			view.unmount();
		} finally {
			HTMLElement.prototype.getBoundingClientRect = original;
			globalThis.requestAnimationFrame = realFrame;
		}
	});
});
