/**
 * El botón de cada ventana guarda un tamaño mínimo (vasak-desktop#209).
 *
 * La píldora de ventanas cede su ancho a las vecinas (`min-w-0`) y desplaza
 * adentro lo que no entra (`overflow-x-auto`). Sin fijar los botones, cuando
 * faltaba lugar —barra compacta, muchas ventanas o ventana angosta— lo que
 * cedía eran los botones: se encogían hasta el relleno y el icono, con el
 * `max-width:100%` del preflight, se aplastaba a cero y quedaba ilegible.
 *
 * Se comprueba sobre la `TrayIconButton` **publicada** (no se dobla): que el
 * botón reciba `shrink-0` para no encogerse y un tamaño mínimo (`min-w-8`,
 * `min-h-8`, el blanco táctil de 32 px) para que el icono no colapse.
 */
import { beforeAll, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { loadComponent, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'window-button-doubles.ts');
const dom = useDom();

let WindowPanelButton: any;

beforeAll(async () => {
	WindowPanelButton = await loadComponent(
		dom.workdir(),
		'src/components/buttons/WindowPanelButton.vue',
		DOUBLES,
		'WindowPanelButton'
	);
}, 60_000);

const settle = async () => {
	for (let i = 0; i < 4; i++) await nextTick();
};

async function render(props: Record<string, unknown>) {
	const view = mount(WindowPanelButton as any, { props, attachTo: document.body });
	await settle();
	return view;
}

describe('el botón de ventana del panel', () => {
	test('no se encoge y guarda un tamaño mínimo, para que el icono no colapse', async () => {
		const view = await render({ id: '1', title: 'Firefox', is_minimized: false, icon: 'firefox' });
		const button = view.find('button');

		expect(button.exists()).toBe(true);
		expect(button.classes()).toContain('shrink-0');
		expect(button.classes()).toContain('min-w-8');
		expect(button.classes()).toContain('min-h-8');
		view.unmount();
	});

	test('el tamaño mínimo también está en una ventana minimizada', async () => {
		const view = await render({ id: '2', title: 'Terminal', is_minimized: true, icon: 'utilities-terminal' });
		const button = view.find('button');

		expect(button.exists()).toBe(true);
		expect(button.classes()).toContain('shrink-0');
		expect(button.classes()).toContain('min-w-8');
		expect(button.classes()).toContain('min-h-8');
		view.unmount();
	});
});
