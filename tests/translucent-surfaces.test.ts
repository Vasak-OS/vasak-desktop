import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Las superficies del escritorio son translúcidas, y sin `backdrop-blur`.
 *
 * Con 1.21–1.23 el panel, el menú, los applets y los widgets quedaron en
 * `ui-float`, que es opaco: tapaba el desenfoque que pone Wayfire detrás de la
 * superficie de capa (corrección del usuario del 02/10/2026). Cada raíz de
 * superficie lleva `bg-ui-shell` —el fondo de la ventana translúcido, de
 * vue-libvasak 2.3.0— o un fondo con opacidad explícita (`bg-x/NN`), y nunca
 * uno opaco. El `backdrop-blur` sigue prohibido en todo `src/` por la guardia
 * del diseño; acá se mira además en cada raíz.
 */
const ROOT = join(import.meta.dir, '..');
const read = (file: string) => readFileSync(join(ROOT, file), 'utf8');

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

/** Los fondos que nombra un trozo de plantilla (`bg-…`), sin variantes de estado. */
function backgroundsOf(classes: string): string[] {
	return [...classes.matchAll(/(?<![\w:/-])bg-([a-z][\w-]*(?:\/\d+)?)(?![\w/-])/g)].map((match) => match[1] as string);
}

/** Un fondo deja ver lo de atrás si es `ui-shell`, `transparent` o lleva `/NN` < 100. */
function isTranslucent(background: string): boolean {
	if (background === 'ui-shell' || background === 'transparent') return true;
	const alpha = background.match(/\/(\d+)$/)?.[1];
	return alpha !== undefined && Number(alpha) < 100;
}

/**
 * La raíz de cada superficie: el archivo y la marca que la identifica dentro de
 * él. Lo que se lee es el atributo de clases del elemento que la lleva.
 */
const SURFACES: Array<[string, string, RegExp]> = [
	['el panel', 'src/views/PanelView.vue', /<nav\b[\s\S]*?class="([^"]*)"/],
	['el menú y los applets', 'src/components/layouts/AppletPopover.vue', /'(applet-popover [^']*)'/],
	['el centro de control', 'src/views/ControlCenterView.vue', /'([^']*h-screen w-screen[^']*)'/],
	['el menú de Connect', 'src/views/ConnectMenuView.vue', /'(flex h-screen[^']*)'/],
	['el OSD', 'src/views/apps/OsdPopupView.vue', /class="(w-screen h-screen[^"]*)"/],
	['la ventana de sesión', 'src/views/apps/SessionPopupView.vue', /'(h-screen w-screen[^']*)'/],
	['el marco de los widgets', 'src/components/widgets/WidgetFrame.vue', /surface === 'shell' \? '([^']*)' : '([^']*)'/],
	['la paleta de widgets', 'src/components/widgets/WidgetLayer.vue', /<aside\b[\s\S]*?class="([^"]*)"/],
];

/** Lo que mira la prueba, para poder probar la prueba. */
function surfaceProblems(classes: string[]): string[] {
	const problems: string[] = [];
	for (const group of classes) {
		const backgrounds = backgroundsOf(group);
		if (backgrounds.length === 0) problems.push(`sin fondo: «${group}»`);
		for (const background of backgrounds) {
			if (!isTranslucent(background)) problems.push(`opaco: bg-${background}`);
		}
		if (/backdrop-blur/.test(group)) problems.push('con backdrop-blur');
	}
	return problems;
}

describe('las superficies del escritorio dejan ver el desenfoque de Wayfire', () => {
	test.each(SURFACES)('%s', (_name, file, marker) => {
		const match = template(file).match(marker);
		expect(match, `no se encontró la raíz en ${file}`).not.toBeNull();
		const classes = (match as RegExpMatchArray).slice(1).filter(Boolean);
		expect(surfaceProblems(classes)).toEqual([]);
	});

	test('el marco suelto del menú (el clima) también es translúcido', () => {
		expect(template('src/components/widgets/WidgetSlot.vue')).toContain('<WidgetFrame surface="surface"');
	});

	test('ui-shell existe y es translúcida en la librería instalada', () => {
		const tokens = read('node_modules/@vasakgroup/vue-libvasak/dist/tokens.css');
		const shell = tokens.match(/--color-ui-shell:\s*([^;]+);/)?.[1] ?? '';
		expect(shell).toMatch(/^color-mix\(in srgb, var\(--use-ui-background\) (\d+)%, transparent\)$/);
		expect(Number(shell.match(/(\d+)%/)?.[1])).toBeLessThan(100);
	});
});

describe('la guardia de translucidez ve lo opaco cuando lo hay', () => {
	test('rechaza los fondos con los que quedó 1.23.0', () => {
		expect(surfaceProblems(['rounded-corner-l bg-ui-float border border-ui-line'])).toEqual(['opaco: bg-ui-float']);
		expect(surfaceProblems(['h-screen bg-ui-bg border'])).toEqual(['opaco: bg-ui-bg']);
		expect(surfaceProblems(['bg-ui-surface p-2'])).toEqual(['opaco: bg-ui-surface']);
		expect(surfaceProblems(['bg-ui-bg/80 backdrop-blur-md'])).toEqual(['con backdrop-blur']);
		expect(surfaceProblems(['h-screen border'])).toHaveLength(1);
	});

	test('y deja pasar los translúcidos', () => {
		expect(surfaceProblems(['bg-ui-shell shadow-surface-l', 'bg-ui-surface/70', 'bg-ui-bg/80 hover:bg-ui-hover'])).toEqual([]);
	});
});
