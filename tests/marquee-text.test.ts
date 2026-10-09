/**
 * `MarqueeText`: el nombre a ancho de su celda que se desplaza sólo si no entra
 * (vasak-desktop#203).
 *
 * El cálculo de cuánto sobra es un ayudante puro (`marquee.ts`), que se prueba
 * sin DOM; el componente se compila de verdad y se monta con el ítem publicado,
 * para ver que nunca excede su hueco —`w-full` y `overflow-hidden`— por más largo
 * que sea el nombre, que es lo que rompía la grilla.
 */
import { beforeAll, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { marqueeOverflows, marqueeShift } from '../src/tools/marquee';
import { loadComponent, useDom } from './support/mount-sfc';

describe('el cálculo del desplazamiento', () => {
	test('cuando entra, no sobra nada', () => {
		expect(marqueeShift(80, 120)).toBe(0);
		expect(marqueeShift(120, 120)).toBe(0);
		expect(marqueeOverflows(80, 120)).toBe(false);
	});

	test('cuando no entra, sobra la diferencia', () => {
		expect(marqueeShift(200, 120)).toBe(80);
		expect(marqueeOverflows(200, 120)).toBe(true);
	});

	test('un sobrante de un píxel se ignora: es el redondeo del subpíxel', () => {
		expect(marqueeShift(121, 120)).toBe(0);
		expect(marqueeOverflows(121, 120)).toBe(false);
		expect(marqueeShift(122, 120)).toBe(2);
	});
});

describe('el componente', () => {
	const DOUBLES = join(import.meta.dir, 'support', 'marquee-doubles.ts');
	const dom = useDom();
	let Marquee: any;

	beforeAll(async () => {
		Marquee = await loadComponent(
			dom.workdir(),
			'src/components/MarqueeText.vue',
			DOUBLES,
			'MarqueeText'
		);
	}, 60_000);

	test('muestra el nombre y lo pone en el globo', async () => {
		const view = mount(Marquee, { props: { text: 'Firefox' }, attachTo: document.body });
		await nextTick();
		expect(view.text()).toContain('Firefox');
		expect(view.get('[data-marquee]').attributes('title')).toBe('Firefox');
		view.unmount();
	});

	test('la caja nunca excede su celda, por más largo que sea el nombre', async () => {
		const long = 'UnNombreLarguísimoSinEspaciosQueRomperíaLaGrillaSinEstoEncima';
		const view = mount(Marquee, { props: { text: long }, attachTo: document.body });
		await nextTick();
		const box = view.get('[data-marquee]');
		// `w-full` + `overflow-hidden`: no puede empujar ni desbordar la celda.
		expect(box.classes()).toContain('w-full');
		expect(box.classes()).toContain('overflow-hidden');
		expect(box.classes()).toContain('block');
		// El nombre entero sigue disponible para leerlo (globo y contenido).
		expect(box.attributes('title')).toBe(long);
		expect(view.get('[data-marquee-inner]').text()).toBe(long);
		view.unmount();
	});
});
