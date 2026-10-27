/**
 * Cambiar de categoría en el menú anima (§5.5 de once-ui).
 *
 * En los esqueletos clásico y compacto, `MenuArea` cambiaba las aplicaciones de
 * la categoría elegida sin ninguna transición. Ahora la lista entera cruza en
 * una sola animación —el mismo `RevealTransition out-in` que usa la búsqueda—,
 * identificada por la categoría: una por cambio de sección, no una por app.
 */
import { beforeAll, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';

const DOUBLES = join(import.meta.dir, 'support', 'menu-area-doubles.ts');
const read = (file: string) => readFileSync(join(ROOT, file), 'utf8');
const dom = useDom();
let MenuArea: any;

beforeAll(async () => {
	MenuArea = await loadComponent(dom.workdir(), 'src/components/areas/menu/MenuArea.vue', DOUBLES, 'MenuArea');
}, 60_000);

const app = (name: string) => ({ name, description: '', path: `/${name}`, icon: name });

describe('MenuArea cruza la lista al cambiar de categoría', () => {
	test('envuelve la lista en un RevealTransition out-in, con la categoría de clave', () => {
		const source = read('src/components/areas/menu/MenuArea.vue');
		expect(source).toMatch(/<RevealTransition\s+mode="out-in">/);
		expect(source).toMatch(/<div\s+:key="sectionKey"\s+role="menu"/);
		expect(source).toMatch(/sectionKey:\s*\{/);
	});

	test('al cambiar la categoría, la lista entera es otro nodo (dispara la transición)', async () => {
		const view = mount(MenuArea, { props: { apps: [app('Firefox')], sectionKey: 'all' } });
		const before = view.find('[role="menu"]').element;

		await view.setProps({ apps: [app('Kate')], sectionKey: 'office' });
		await nextTick();

		expect(view.find('[role="menu"]').element).not.toBe(before);
		expect(view.text()).toContain('Kate');
		view.unmount();
	});

	test('si sólo cambian las apps y no la categoría, la lista se conserva', async () => {
		const view = mount(MenuArea, { props: { apps: [app('Firefox')], sectionKey: 'all' } });
		const before = view.find('[role="menu"]').element;

		await view.setProps({ apps: [app('Firefox'), app('Kate')] });
		await nextTick();

		expect(view.find('[role="menu"]').element).toBe(before);
		view.unmount();
	});
});

describe('los esqueletos clásico y compacto le pasan la categoría', () => {
	for (const file of ['ClassicMenu', 'CompactMenu']) {
		test(`${file} pasa :section-key="categorySelected" a MenuArea`, () => {
			expect(read(`src/components/areas/menu/layouts/${file}.vue`)).toMatch(
				/<MenuArea\s+:apps="appsOfCategory"\s+:section-key="categorySelected"\s*\/>/
			);
		});
	}
});
