/**
 * La píldora de Lince en el panel (`PanelView.vue`): existe sólo mientras el
 * asistente está encendido (`assistant_available`).
 */
import { beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { panelAssistant } from './support/panel-assistant-doubles';
import { loadView } from './support/load-view';
import { useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'panel-assistant-doubles.ts');
const dom = useDom();

let Panel: any;
beforeAll(async () => {
	Panel = await loadView(dom.workdir(), 'src/views/PanelView.vue', DOUBLES, 'PanelView');
}, 60_000);

beforeEach(() => panelAssistant.reset());

async function render() {
	const view = mount(Panel, { attachTo: document.body });
	for (let i = 0; i < 6; i++) {
		await Promise.resolve();
		await nextTick();
	}
	return view;
}

describe('la píldora del asistente en el panel', () => {
	test('con el asistente encendido aparece, y tocarla lo abre', async () => {
		const view = await render();
		const pill = view.find('[data-assistant-pill]');

		expect(pill.exists()).toBe(true);
		await pill.trigger('click');
		expect(panelAssistant.calls).toEqual(['toggleAssistant']);
		view.unmount();
	});

	test('con el asistente apagado no está', async () => {
		panelAssistant.available = async () => false;
		const view = await render();

		expect(view.find('[data-assistant-pill]').exists()).toBe(false);
		view.unmount();
	});

	test('si no se puede saber si está encendido, no está (apagado se ve apagado)', async () => {
		panelAssistant.available = () => Promise.reject(new Error('sin bus'));
		const view = await render();

		expect(view.find('[data-assistant-pill]').exists()).toBe(false);
		view.unmount();
	});
});
