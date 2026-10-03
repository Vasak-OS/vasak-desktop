/**
 * Montar un `.vue` del escritorio con los servicios cambiados por dobles.
 *
 * El DOM lo pone el `preload` (`dom.ts`). Los dobles de Tauri no: cambiarían
 * el ambiente de las otras trescientas pruebas, así que `useDom()` pone un
 * `__TAURI_INTERNALS__` de mentira en `beforeAll` y lo saca en `afterAll`.
 *
 * El componente se compila de verdad (`compileScript` con la plantilla
 * adentro) y cada `import … from '@/…'` se manda a un módulo de dobles que
 * escribe la prueba. Es lo mismo que hace `menu-app-row.test.ts`, y por lo
 * mismo: con `mock.module` el doble queda en la caché de módulos y le llega a
 * otro archivo según el orden en que corran. Los componentes de la librería no
 * se doblan: son los publicados.
 */
import { afterAll, beforeAll } from 'bun:test';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { compileScript, parse } from 'vue/compiler-sfc';

export const ROOT = join(import.meta.dir, '..', '..');

/** Lo que el `invoke` de mentira contesta, y lo que le pidieron. */
export const tauriCalls: Array<{ cmd: string; args: unknown }> = [];

export function useDom(): { workdir: () => string } {
	let workdir = '';

	beforeAll(() => {
		(globalThis as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
			metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
			transformCallback: () => 0,
			convertFileSrc: (path: string) => path,
			invoke: async (cmd: string, args: unknown) => {
				tauriCalls.push({ cmd, args });
				// Los iconos del tema y el catálogo: vacíos, que la prueba no mira píxeles.
				if (cmd === 'plugin:i18n|load_translations') return {};
				if (cmd === 'plugin:i18n|get_locale') return 'es';
				if (cmd === 'plugin:event|listen') return 1;
				return '';
			},
		};
		// Adentro del repositorio, para que `vue` y la librería se resuelvan
		// desde el `node_modules` de acá: en /tmp no hay de dónde sacarlos.
		const cache = join(ROOT, 'node_modules', '.cache');
		mkdirSync(cache, { recursive: true });
		(globalThis as unknown as { __TAURI_EVENT_PLUGIN_INTERNALS__: unknown }).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
			unregisterListener: () => {},
		};
		workdir = mkdtempSync(join(cache, 'sfc-'));
	});

	afterAll(() => {
		delete (globalThis as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
		delete (globalThis as unknown as { __TAURI_EVENT_PLUGIN_INTERNALS__?: unknown }).__TAURI_EVENT_PLUGIN_INTERNALS__;
		rmSync(workdir, { recursive: true, force: true });
	});

	return { workdir: () => workdir };
}

/**
 * Compila `source` (relativa a la raíz) y la importa con sus `@/…` en `doubles`.
 *
 * `doubles` es la ruta absoluta del módulo de dobles; el `default` de ese
 * módulo es lo que recibe un `import X from '@/…vue'`.
 */
export async function loadComponent(workdir: string, source: string, doubles: string, name: string) {
	const file = join(ROOT, source);
	const { descriptor } = parse(await Bun.file(file).text(), { filename: file });
	const compiled = compileScript(descriptor, { id: name, inlineTemplate: true });
	const code = compiled.content.replace(/from\s+(['"])@\/[^'"]+\1/g, `from ${JSON.stringify(doubles)}`);
	const out = join(workdir, `${name}.ts`);
	writeFileSync(out, code);
	return (await import(out)).default;
}
