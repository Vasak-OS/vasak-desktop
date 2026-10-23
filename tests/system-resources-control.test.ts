/**
 * La píldora de recursos del panel (vasak-desktop#216): CPU, memoria y
 * temperatura que el applet de Rust empuja por `system-resources`.
 *
 * La cuenta (`cpuPercent`, `memoryPercent`) ya se prueba en
 * `widget-sistema.test.ts`; acá se prueba lo que hace el componente con ella.
 * El clic se mira en lo que se le pidió al plugin de shell (`tauriCalls`).
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import type { SystemStats } from '../src/tools/system-stats';
import { loadComponent, tauriCalls, useDom } from './support/mount-sfc';
import { resources } from './support/system-resources-doubles';

const DOUBLES = join(import.meta.dir, 'support', 'system-resources-doubles.ts');
const dom = useDom();

let Control: any;

beforeAll(async () => {
	Control = await loadComponent(
		dom.workdir(),
		'src/components/controls/SystemResourceControl.vue',
		DOUBLES,
		'SystemResourceControl'
	);
}, 60_000);

beforeEach(() => {
	resources.reset();
	tauriCalls.length = 0;
});

const lectura = (over: Partial<SystemStats> = {}): SystemStats => ({
	cpu_total: 1000,
	cpu_idle: 800,
	mem_total_kib: 16_000_000,
	mem_available_kib: 4_000_000,
	temperature_c: 50,
	...over,
});

const settle = async () => {
	for (let i = 0; i < 4; i++) await nextTick();
};

async function render() {
	const view = mount(Control, { attachTo: document.body });
	await settle();
	return view;
}

async function emit(stats: SystemStats) {
	resources.emit('system-resources', stats);
	await settle();
}

describe('la píldora de recursos', () => {
	test('no dibuja nada hasta la primera medición', async () => {
		const view = await render();
		expect(view.find('button').exists()).toBe(false);
		view.unmount();
	});

	test('con la primera medición aparece, con la memoria en uso y la CPU en cero', async () => {
		const view = await render();
		await emit(lectura());

		expect(view.find('button').exists()).toBe(true);
		const text = view.text();
		expect(text).toContain('75%');
		expect(text).toContain('0%');
		view.unmount();
	});

	test('la CPU sale de la diferencia con la lectura anterior', async () => {
		const view = await render();
		await emit(lectura());
		await emit(lectura({ cpu_total: 2000, cpu_idle: 1300 }));

		expect(view.text()).toContain('50%');
		view.unmount();
	});

	test('la fila de temperatura aparece con sensor, redondeada, y se va sin él', async () => {
		const view = await render();
		await emit(lectura({ temperature_c: 61.6 }));
		expect(view.text()).toContain('62°');

		await emit(lectura({ temperature_c: null }));
		expect(view.text()).not.toContain('°');
		view.unmount();
	});

	test('una máquina sin sensor no muestra temperatura desde la primera lectura', async () => {
		const view = await render();
		await emit(lectura({ temperature_c: null }));

		expect(view.find('button').exists()).toBe(true);
		expect(view.text()).not.toContain('°');
		view.unmount();
	});

	test('un clic abre vasak-monitor', async () => {
		const view = await render();
		await emit(lectura());
		await view.find('button').trigger('click');
		await settle();

		const spawn = tauriCalls.find((call) => call.cmd === 'plugin:shell|spawn');
		expect(spawn?.args).toMatchObject({ program: 'vasak-monitor' });
		view.unmount();
	});
});
