/**
 * La salida de las ventanas flotantes tras adoptar `RevealTransition`
 * (vasak-desktop, vue-libvasak 2.21.0).
 *
 * Antes la salida era una clase `leave-active` con un `@keyframes` propio; ahora
 * la vista se esconde con `v-show="!leaving"` y la leave de la transición corre
 * antes de que el temporizador de 200 ms oculte la ventana. Lo que se comprueba
 * es el comportamiento: Escape saca el contenido, y reabrir lo trae de vuelta.
 * El fundido y la escala en sí se ven en WebKitGTK, no acá.
 */
import { beforeAll, describe, expect, test } from 'bun:test';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { createMemoryHistory, createRouter } from 'vue-router';
import { compileScript, parse } from 'vue/compiler-sfc';
import { sharedEvents } from './support/calendar-doubles';
import { loadComponent, ROOT, useDom } from './support/mount-sfc';

const dom = useDom();
const CALENDAR_DOUBLES = join(import.meta.dir, 'support', 'calendar-doubles.ts');
const DOUBLES = join(import.meta.dir, 'support', 'transiciones-doubles.ts');

/** Espera, sin dormir a ciegas, a que `condition` valga (la leave termina en un par de cuadros). */
async function until(condition: () => boolean) {
	for (let i = 0; i < 200 && !condition(); i++) {
		await nextTick();
		await new Promise((resolve) => setTimeout(resolve, 5));
	}
}

const pressEscape = () =>
	document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));

/** Escape hasta que se esconde: el `keydown` se engancha en `onMounted`, tras unos `await`. Es idempotente. */
const escapeUntilHidden = (root: { element: Element }) =>
	until(() => {
		pressEscape();
		return hidden(root);
	});

const hidden = (root: { element: Element }) => (root.element as HTMLElement).style.display === 'none';

describe('la ventana de sesión', () => {
	let Popup: any;
	beforeAll(async () => {
		Popup = await loadComponent(dom.workdir(), 'src/views/apps/SessionPopupView.vue', DOUBLES, 'SessionPopupView');
	}, 60_000);

	test('Escape esconde el contenido por `v-show` y no desmonta la vista', async () => {
		const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/', component: {} }] });
		await router.push('/');
		const view = mount(Popup, { attachTo: document.body, global: { plugins: [router] } });
		await nextTick();
		const card = view.find('.rounded-corner-window');
		expect(card.exists()).toBe(true);
		expect(hidden(card)).toBe(false);

		await escapeUntilHidden(card);

		// Sigue en el DOM, sólo escondida: la ventana se oculta, no se destruye.
		expect(view.find('.rounded-corner-window').exists()).toBe(true);
		expect(hidden(card)).toBe(true);
		view.unmount();
	});
});

describe('el menú de Connect', () => {
	let Menu: any;
	beforeAll(async () => {
		Menu = await loadComponent(dom.workdir(), 'src/views/ConnectMenuView.vue', DOUBLES, 'ConnectMenuView');
	}, 60_000);

	test('Escape esconde el contenido por `v-show` y no desmonta la vista', async () => {
		const view = mount(Menu, { attachTo: document.body });
		await until(() => view.find('.window-border').exists());
		const card = view.find('.window-border');
		expect(hidden(card)).toBe(false);

		await escapeUntilHidden(card);

		expect(view.find('.window-border').exists()).toBe(true);
		expect(hidden(card)).toBe(true);
		view.unmount();
	});
});

describe('el centro de control', () => {
	let Center: any;
	beforeAll(async () => {
		// Los componentes del escritorio, doblados: lo que se mira es la vista.
		const source = 'src/views/ControlCenterView.vue';
		const file = join(ROOT, source);
		const { descriptor } = parse(readFileSync(file, 'utf8'), { filename: file });
		const compiled = compileScript(descriptor, { id: 'ControlCenterView', inlineTemplate: true });
		const code = `import * as __doubles from ${JSON.stringify(CALENDAR_DOUBLES)};\n${compiled.content
			.replace(/import\s+type\s+\{[^}]*\}\s+from\s+(['"])@\/[^'"]+\1;?/g, '')
			.replace(/import\s+(\w+)\s+from\s+(['"])@\/[^'"]+\.vue\2;?/g, "const $1 = __doubles.$1 ?? __doubles.silent('$1');")
			.replace(
				/import\s+\{([^}]*)\}\s+from\s+(['"])@\/[^'"]+\2;?/g,
				(_all, names: string) => `const {${names.replace(/\btype\s+\w+\s*,?/g, '').replace(/\s+as\s+/g, ': ')}} = __doubles;`
			)}`;
		const out = join(dom.workdir(), 'ControlCenterTransition.ts');
		writeFileSync(out, code);
		Center = (await import(out)).default;
	}, 60_000);

	test('Escape lo esconde y al mostrarse de nuevo vuelve a verse', async () => {
		const view = mount(Center, { attachTo: document.body });
		await until(() => view.find('main').exists());
		const main = view.find('main');
		expect(hidden(main)).toBe(false);

		await escapeUntilHidden(main);
		expect(hidden(main)).toBe(true);

		// La ventana no se destruye: `window-shown` desarma `leaving` y entra otra vez.
		sharedEvents.get('window-shown')?.();
		await until(() => !hidden(main));
		expect(hidden(main)).toBe(false);
		view.unmount();
	});
});
