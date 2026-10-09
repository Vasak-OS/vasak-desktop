import { describe, expect, test } from 'bun:test';
import { Glob } from 'bun';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * El borde de afuera del escritorio (vue-libvasak 2.16.0).
 *
 * Lo que da al escritorio —el panel, el centro de control, los applets
 * anclados, el menú de Connect, el OSD, el emergente de sesión y el cajón de
 * widgets— lleva `window-border`: grosor (`--window-border-width`) y color
 * (`--ui-window-border`) los elige la persona en Configuración y los escribe el
 * config-manager. Lo de adentro —tarjetas, secciones, separadores— sigue en
 * `border border-ui-line`: el grosor y el acento son del marco, no de cada caja.
 *
 * Se mira la clase pedida en la plantilla: Tailwind arma la regla desde ahí, y
 * una superficie que vuelve a `border border-ui-line` ignora lo elegido sin que
 * nada se rompa a la vista. La franja del panel se prueba en
 * `src/tools/panel-appearance.test.ts`, en cada tipo y cada lado.
 */

const SRC = join(import.meta.dir, '..', 'src');
const read = (file: string) => readFileSync(join(SRC, file), 'utf8');

/** Saca los comentarios HTML cortando por sus delimitadores. */
function stripHtmlComments(text: string): string {
	let out = '';
	let index = 0;
	while (index < text.length) {
		const start = text.indexOf('<!--', index);
		if (start === -1) return out + text.slice(index);
		out += text.slice(index, start);
		const end = text.indexOf('-->', start + 4);
		if (end === -1) return out;
		index = end + 3;
	}
	return out;
}

const template = (file: string) => {
	const text = read(file);
	return stripHtmlComments(text.slice(text.indexOf('<template>'), text.lastIndexOf('</template>')));
};

/** Las clases del texto entre comillas que contiene `anchor`, como lista. */
function classesAround(file: string, anchor: string): string[] {
	const view = template(file);
	const at = view.indexOf(anchor);
	expect(at, `${file}: no aparece «${anchor}»`).toBeGreaterThan(-1);
	const quotes = /["']/g;
	let start = -1;
	for (let i = at; i >= 0; i--) {
		if (view[i] === '"' || view[i] === "'") {
			start = i;
			break;
		}
	}
	quotes.lastIndex = at;
	const end = quotes.exec(view)?.index ?? view.length;
	return view.slice(start + 1, end).split(/\s+/).filter(Boolean);
}

/** Lo de afuera: archivo y un pedazo de la clase que identifica la superficie. */
const OUTER: Array<[string, string]> = [
	['views/ControlCenterView.vue', 'bg-ui-shell h-screen w-screen'],
	['views/ConnectMenuView.vue', 'flex h-screen min-w-0 flex-col gap-3'],
	['views/apps/OsdPopupView.vue', 'w-screen h-screen flex flex-col'],
	['views/apps/SessionPopupView.vue', 'h-screen w-screen flex items-center justify-center'],
	['components/widgets/WidgetLayer.vue', 'max-h-[40vh]'],
	['components/layouts/AppletPopover.vue', 'rounded-corner-xl'],
];

/** Lo de adentro: tarjetas y aros que siguen con el canto del esquema. */
const INNER: Array<[string, string]> = [
	['views/applets/ScreenTimeAppletView.vue', 'flex min-w-0 flex-col items-center justify-center gap-1 rounded-corner-l'],
	['views/applets/ScreenTimeAppletView.vue', 'flex shrink-0 flex-col gap-1 rounded-corner-l'],
	['views/applets/DateBoardAppletView.vue', 'min-w-0 rounded-corner-l border'],
	['views/applets/DateBoardAppletView.vue', 'flex min-w-0 flex-col gap-3 rounded-corner-l'],
	['views/apps/SessionPopupView.vue', 'w-20 h-20 rounded-corner-full'],
];

describe('lo de afuera lleva el borde que eligió la persona', () => {
	test.each(OUTER)('%s', (file, anchor) => {
		const classes = classesAround(file, anchor);
		expect(classes).toContain('window-border');
		// Ni el canto fijo de antes: con los dos, gana el que Tailwind emita
		// después y lo elegido en Configuración puede no verse.
		expect(classes).not.toContain('border');
		expect(classes).not.toContain('border-ui-line');
	});

	test('el overlay a pantalla completa sigue sin borde', () => {
		// No hay afuera donde se vea (vasak-desktop#210).
		expect(template('components/layouts/AppletPopover.vue')).toMatch(/full\s*\?\s*'applet-popover-full'\s*:/);
	});
});

describe('lo de adentro sigue con el canto del esquema', () => {
	test.each(INNER)('%s', (file, anchor) => {
		const classes = classesAround(file, anchor);
		expect(classes).toContain('border');
		expect(classes).toContain('border-ui-line');
		expect(classes).not.toContain('window-border');
	});

	test('`window-border` aparece sólo en las superficies de afuera', () => {
		// El grosor y el acento son del marco: una tarjeta con `window-border`
		// se pondría de 2 px y del primario cuando la persona lo pide para la
		// ventana.
		const allowed = new Set([...OUTER.map(([file]) => file), 'tools/panel-appearance.ts']);
		const users = [...new Glob('**/*.{vue,ts}').scanSync(SRC)]
			.filter((file) => !file.endsWith('.test.ts'))
			.filter((file) => /(?<![\w-])window-border(?![\w-])/.test(stripHtmlComments(read(file))))
			.sort();
		expect(users).toEqual([...allowed].sort());
	});
});
