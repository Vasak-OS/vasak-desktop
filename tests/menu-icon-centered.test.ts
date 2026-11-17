import { afterAll, beforeAll, describe, expect, test } from 'bun:test';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { createSSRApp, h } from 'vue';
import { compileScript, parse } from 'vue/compiler-sfc';
import { renderToString } from 'vue/server-renderer';

/**
 * En los favoritos y en los mosaicos el icono va arriba y el nombre abajo.
 *
 * `ThemeIcon` dibuja un `<img>`, y el preflight de Tailwind lo pone en
 * `display: block`: el `text-center` del ítem sólo centra el texto, así que el
 * icono quedaba pegado a la izquierda de su celda. Se necesita `mx-auto` sobre
 * el icono. Acá se compilan los `.vue` de verdad y se mira el HTML que sale; en
 * SSR `ThemeIcon` sin resolver dibuja el `<span class="inline-block shrink-0">`
 * hueco, y es ése el que tiene que llevar la clase. También se dibuja la
 * plantilla sin `mx-auto` para ver que la prueba la rechaza.
 */
const DIR = join(import.meta.dir, '..', 'src', 'components', 'areas', 'menu');
const APP = { name: 'Firefox', icon: 'firefox', path: '/usr/share/applications/firefox.desktop' };

const CASES = [
	{ name: 'FavoritesArea', file: 'FavoritesArea.vue', props: { apps: [APP] } },
	{ name: 'AppTile', file: 'AppTile.vue', props: { app: APP } },
];

let workdir = '';

async function render(file: string, source: string, props: Record<string, unknown>, id: string): Promise<string> {
	const { descriptor } = parse(source, { filename: file });
	const compiled = compileScript(descriptor, { id, inlineTemplate: true });
	// Los imports `@/` se cambian por dobles, como en `menu-app-row.test.ts`.
	const code = compiled.content.replace(/from\s+(['"])@\/[^'"]+\1/g, "from './doubles'");
	const out = join(workdir, `${id}.ts`);
	writeFileSync(out, code);
	const component = (await import(out)).default;
	return renderToString(createSSRApp({ render: () => h(component, props) }));
}

/** Las clases del hueco que dibuja `ThemeIcon` dentro del ítem. */
function iconClasses(html: string): string[] {
	const match = html.match(/<span[^>]*class="([^"]*inline-block[^"]*)"/);
	return (match?.[1] ?? '').split(/\s+/);
}

const current: Record<string, string> = {};
const old: Record<string, string> = {};

beforeAll(async () => {
	workdir = mkdtempSync(join(import.meta.dir, '..', 'src', '.menu-icon-'));
	writeFileSync(
		join(workdir, 'doubles.ts'),
		[
			"import { defineComponent, h } from 'vue';",
			"export default defineComponent({ props: ['text'], render() { return h('span', this.text); } });",
			'export const useAppLauncher = () => ({ launch: async () => {}, toggleFavoriteFor: async () => {} });',
			'',
		].join('\n')
	);
	for (const c of CASES) {
		const path = join(DIR, c.file);
		const source = readFileSync(path, 'utf8');
		current[c.name] = await render(path, source, c.props, `${c.name}-current`);
		old[c.name] = await render(path, source.replace(/ class="mx-auto"/g, ''), c.props, `${c.name}-old`);
	}
});

afterAll(() => {
	if (workdir) rmSync(workdir, { recursive: true, force: true });
});

describe('el icono apilado del menú queda centrado sobre el nombre', () => {
	for (const c of CASES) {
		test(`${c.name}: el icono lleva mx-auto`, () => {
			expect(current[c.name]).toContain('Firefox');
			expect(iconClasses(current[c.name])).toContain('mx-auto');
		});

		test(`${c.name}: la plantilla sin mx-auto, con el icono a la izquierda, no pasa`, () => {
			expect(old[c.name]).toContain('Firefox');
			expect(iconClasses(old[c.name])).not.toContain('mx-auto');
		});
	}
});
