import { afterEach, beforeAll, describe, expect, test } from 'bun:test';
import { mount } from '@vue/test-utils';
import { defineComponent, h, nextTick, ref } from 'vue';
import { useBoxSize } from '../src/tools/composables/useBoxSize';
import { todayIso } from '../src/tools/calendar-widget';
import { useDom } from './support/mount-sfc';

useDom();

let notify: (() => void) | null = null;
let disconnected = false;

beforeAll(() => {
	(globalThis as unknown as { ResizeObserver: unknown }).ResizeObserver = class {
		constructor(callback: () => void) {
			notify = callback;
		}
		observe() {}
		disconnect() {
			disconnected = true;
		}
	};
});

afterEach(() => {
	delete (HTMLElement.prototype as unknown as Record<string, unknown>).clientWidth;
	delete (HTMLElement.prototype as unknown as Record<string, unknown>).clientHeight;
	notify = null;
	disconnected = false;
});

function mountBox(size: { w: number; h: number }) {
	Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, get: () => size.w });
	Object.defineProperty(HTMLElement.prototype, 'clientHeight', { configurable: true, get: () => size.h });
	let measured!: ReturnType<typeof useBoxSize>;
	const view = mount(
		defineComponent({
			setup() {
				const box = ref<HTMLElement | null>(null);
				measured = useBoxSize(box);
				return () => h('div', { ref: box });
			},
		}),
		{ attachTo: document.body }
	);
	return { view, measured };
}

describe('useBoxSize', () => {
	test('mide al montar y vuelve a medir cuando la caja cambia', async () => {
		const size = { w: 400, h: 120 };
		const { view, measured } = mountBox(size);
		await nextTick();
		expect([measured.width.value, measured.height.value]).toEqual([400, 120]);

		size.w = 260;
		size.h = 384;
		notify?.();
		expect([measured.width.value, measured.height.value]).toEqual([260, 384]);
		view.unmount();
	});

	test('deja de observar al desmontar', () => {
		const { view } = mountBox({ w: 1, h: 1 });
		view.unmount();
		expect(disconnected).toBe(true);
	});
});

describe('todayIso', () => {
	test('es la fecha local en formato ISO', () => {
		expect(todayIso(new Date(2026, 9, 21, 23, 59))).toBe('2026-10-21');
	});
});
