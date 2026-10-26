/**
 * La superposición del historial del portapapeles (vasak-desktop#217).
 *
 * Se monta con los servicios de verdad y el `invoke` de mentira: lo que se mira
 * es qué comando le llega al backend. Las imágenes y la persistencia quedan para
 * después; v1 es sólo texto en memoria.
 */
import { afterEach, beforeAll, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { loadComponent, ROOT, tauriCalls, useDom } from './support/mount-sfc';
import { logged, shownHandlers } from './support/clipboard-doubles';

const DOUBLES = join(import.meta.dir, 'support', 'clipboard-doubles.ts');
const dom = useDom();
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

type Internals = { invoke: (cmd: string, args: unknown) => Promise<unknown> };

/** Lo que el backend de mentira tiene en el historial. La prueba lo cambia. */
let history: Array<{ id: number; text: string }> = [];

const settle = async () => {
	for (let i = 0; i < 6; i++) await nextTick();
	await new Promise((resolve) => setTimeout(resolve, 0));
	for (let i = 0; i < 6; i++) await nextTick();
};

const commands = () => tauriCalls.map((call) => call.cmd).filter((cmd) => !cmd.startsWith('plugin:'));

let View: any;
beforeAll(async () => {
	const internals = (globalThis as unknown as { __TAURI_INTERNALS__: Internals }).__TAURI_INTERNALS__;
	const original = internals.invoke;
	internals.invoke = async (cmd, args) => {
		if (cmd === 'clipboard_history') {
			tauriCalls.push({ cmd, args });
			return history;
		}
		return original(cmd, args);
	};
	View = await loadComponent(dom.workdir(), 'src/views/ClipboardView.vue', DOUBLES, 'ClipboardView');
}, 60_000);

afterEach(() => {
	tauriCalls.length = 0;
	logged.length = 0;
	shownHandlers.length = 0;
	history = [];
	document.body.innerHTML = '';
});

async function mountView() {
	const view = mount(View, { attachTo: document.body });
	await settle();
	return view;
}

const rows = (view: Awaited<ReturnType<typeof mountView>>) => view.findAll('[data-list] [role="button"]');

describe('al abrirse pide el historial', () => {
	test('lo pide al montar', async () => {
		history = [{ id: 1, text: 'hola' }];
		const view = await mountView();
		expect(commands()).toContain('clipboard_history');
		expect(view.text()).toContain('hola');
		view.unmount();
	});

	test('lo vuelve a pedir en cada `window-shown` (la ventana se esconde, no se destruye)', async () => {
		const view = await mountView();
		tauriCalls.length = 0;
		history = [{ id: 7, text: 'copiado nuevo' }];
		for (const handler of shownHandlers) handler();
		await settle();
		expect(commands()).toContain('clipboard_history');
		expect(view.text()).toContain('copiado nuevo');
		view.unmount();
	});
});

describe('la lista y el filtro', () => {
	test('sin nada muestra el estado vacío y ninguna fila', async () => {
		history = [];
		const view = await mountView();
		expect(rows(view)).toHaveLength(0);
		expect(view.find('[data-list]').exists()).toBe(false);
		expect(view.find('[data-clear]').exists()).toBe(false);
		view.unmount();
	});

	test('muestra una fila por entrada', async () => {
		history = [
			{ id: 2, text: 'segundo' },
			{ id: 1, text: 'primero' },
		];
		const view = await mountView();
		expect(rows(view)).toHaveLength(2);
		view.unmount();
	});
});

describe('elegir una entrada', () => {
	test('la pone en el portapapeles y cierra, en ese orden', async () => {
		history = [{ id: 42, text: 'para pegar' }];
		const view = await mountView();

		await rows(view)[0].trigger('click');
		await settle();

		const ownCommands = commands().filter((cmd) => cmd !== 'clipboard_history');
		expect(ownCommands).toEqual(['clipboard_pick', 'hide_clipboard']);
		const pick = tauriCalls.find((call) => call.cmd === 'clipboard_pick');
		expect(pick?.args).toEqual({ id: 42 });
		view.unmount();
	});
});

describe('limpiar el historial', () => {
	test('pide el borrado y vacía la vista', async () => {
		history = [{ id: 1, text: 'algo' }];
		const view = await mountView();

		await view.find('[data-clear]').trigger('click');
		await settle();

		expect(commands()).toContain('clipboard_clear');
		expect(rows(view)).toHaveLength(0);
		view.unmount();
	});
});

describe('el backend y los textos', () => {
	test('los comandos están registrados en lib.rs', () => {
		const lib = read('src-tauri/src/lib.rs');
		for (const command of [
			'toggle_clipboard',
			'hide_clipboard',
			'clipboard::clipboard_history',
			'clipboard::clipboard_pick',
			'clipboard::clipboard_clear',
		]) {
			expect(lib, command).toContain(command);
		}
		// El vigía arranca al iniciar la sesión.
		expect(lib).toContain('clipboard::start();');
	});

	test('el atajo entra por el brazo de D-Bus', () => {
		const dbus = read('src-tauri/src/dbus_service.rs');
		expect(dbus).toContain('"ToggleClipboard" | "OpenClipboard"');
	});

	test('los textos existen en los dos idiomas', () => {
		for (const locale of ['es', 'en']) {
			const block = read(`src-tauri/locales/${locale}.yml`);
			const parsed = Bun.YAML.parse(block) as { views: { clipboard: Record<string, string> } };
			expect(parsed.views.clipboard.search, locale).toBeString();
			expect(parsed.views.clipboard.clear, locale).toBeString();
			expect(parsed.views.clipboard.empty, locale).toBeString();
		}
	});
});
