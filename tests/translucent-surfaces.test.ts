import { describe, expect, test } from 'bun:test';
import { Glob } from 'bun';
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
 * uno opaco.
 *
 * El `backdrop-blur` va prohibido en todas las raíces —el desenfoque lo pone
 * Wayfire detrás de cada superficie de capa— con **una sola excepción
 * nombrada: el marco de los widgets del escritorio** (la variante `shell` de
 * `WidgetFrame`). Los widgets se dibujan en la misma página que el fondo de
 * pantalla, así que detrás de ellos Wayfire no tiene nada que desenfocar: el
 * desenfoque lo tiene que hacer el WebView (decisión del usuario, 02/10/2026).
 * Acá se exige que el marco lo tenga y que ninguna otra superficie lo gane.
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

/** Si la superficie desenfoca por su cuenta (`blur`) o lo deja a Wayfire (`none`). */
type Blur = 'blur' | 'none';

/**
 * La raíz de cada superficie: el archivo, la marca que la identifica dentro de
 * él y si desenfoca. Lo que se lee es el atributo de clases del elemento que la
 * lleva. La bandeja es un applet más: su raíz es la de `AppletPopover`.
 */
const SURFACES: Array<[string, string, RegExp, Blur]> = [
	['el panel', 'src/views/PanelView.vue', /<nav\b[\s\S]*?class="([^"]*)"/, 'none'],
	['el menú, los applets y la bandeja', 'src/components/layouts/AppletPopover.vue', /'(applet-popover [^']*)'/, 'none'],
	['el centro de control', 'src/views/ControlCenterView.vue', /'([^']*h-screen w-screen[^']*)'/, 'none'],
	['el menú de Connect', 'src/views/ConnectMenuView.vue', /'(flex h-screen[^']*)'/, 'none'],
	['el OSD', 'src/views/apps/OsdPopupView.vue', /class="(w-screen h-screen[^"]*)"/, 'none'],
	['la ventana de sesión', 'src/views/apps/SessionPopupView.vue', /'(h-screen w-screen[^']*)'/, 'none'],
	// El selector rápido de fondos: sólo un velo detrás de la fila (vasak-desktop#133).
	['el selector de fondos', 'src/views/apps/WallpaperPickerView.vue', /<div\s+class="([^"]*h-screen w-screen[^"]*)"/, 'none'],
	// La excepción: el marco que flota sobre el fondo de pantalla.
	['el marco de los widgets', 'src/components/widgets/WidgetFrame.vue', /surface === 'shell' \? '([^']*)'/, 'blur'],
	// El marco suelto del menú (el clima) va dentro del menú, que ya desenfoca Wayfire.
	['el marco suelto del menú', 'src/components/widgets/WidgetFrame.vue', /surface === 'shell' \? '[^']*' : '([^']*)'/, 'none'],
	['la paleta de widgets', 'src/components/widgets/WidgetLayer.vue', /<aside\b[\s\S]*?class="([^"]*)"/, 'none'],
	// El tablero de fecha va dentro de `AppletPopover` (la raíz de arriba); sus
	// bloques son los de adentro de una superficie: translúcidos, sin desenfoque.
	['el mes del tablero de fecha', 'src/views/applets/DateBoardAppletView.vue', /<section\s+class="([^"]*)"\s+data-date-board-calendar/, 'none'],
	['el clima del tablero de fecha', 'src/views/applets/DateBoardAppletView.vue', /<section\s+class="([^"]*bg-ui-surface[^"]*)"\s+:aria-label="t\('views\.dateBoard\.weather'\)"/, 'none'],
];

/** El único archivo de `src/` que puede nombrar `backdrop-blur`. */
const BLUR_EXCEPTION = 'components/widgets/WidgetFrame.vue';

/** Lo que mira la prueba, para poder probar la prueba. */
function surfaceProblems(classes: string[], blur: Blur = 'none'): string[] {
	const problems: string[] = [];
	for (const group of classes) {
		const backgrounds = backgroundsOf(group);
		if (backgrounds.length === 0) problems.push(`sin fondo: «${group}»`);
		for (const background of backgrounds) {
			if (!isTranslucent(background)) problems.push(`opaco: bg-${background}`);
		}
		const blurred = /(?<![\w-])backdrop-blur-md(?![\w-])/.test(group);
		if (blur === 'none' && /backdrop-blur/.test(group)) problems.push('con backdrop-blur');
		if (blur === 'blur' && !blurred) problems.push('sin backdrop-blur-md');
	}
	return problems;
}

/**
 * Los archivos de `src/` que nombran `backdrop-blur` fuera de un comentario. Sin
 * las pruebas, que lo nombran a propósito.
 */
function filesWithBlur(): string[] {
	return [...new Glob('**/*.{vue,ts,css}').scanSync(join(ROOT, 'src'))]
		.filter((file) => !file.endsWith('.test.ts'))
		.filter((file) => {
			const text = read(`src/${file}`);
			const code = file.endsWith('.vue') ? stripHtmlComments(text) : text;
			return /backdrop-blur/.test(code.replace(/\/\*[\s\S]*?\*\//g, '').replace(/^\s*\/\/.*$/gm, ''));
		})
		.sort();
}

describe('las superficies del escritorio dejan ver el desenfoque de Wayfire', () => {
	test.each(SURFACES)('%s', (_name, file, marker, blur) => {
		const match = template(file).match(marker);
		expect(match, `no se encontró la raíz en ${file}`).not.toBeNull();
		const classes = (match as RegExpMatchArray).slice(1).filter(Boolean);
		expect(classes).toHaveLength(1);
		expect(surfaceProblems(classes, blur)).toEqual([]);
	});

	test('sólo el marco de los widgets desenfoca: ninguna otra pieza de src/ nombra backdrop-blur', () => {
		expect(filesWithBlur()).toEqual([BLUR_EXCEPTION]);
	});

	test('el reproductor desplegable es la superficie de AppletPopover, y nada adentro lo tapa', () => {
		// Hasta 1.23 el selector de salida era una capa `bg-ui-surface` opaca
		// encima de la tarjeta, del tamaño del applet entero: con el selector
		// abierto, el applet dejaba de ser translúcido. Ahora reemplaza a la
		// tarjeta y sus bloques van en `/70`.
		const view = template('src/views/applets/MusicAppletView.vue');
		expect(view.trimStart()).toMatch(/^<template>\s*<AppletPopover applet="music"/);
		expect(backgroundsOf(view).filter((background) => !isTranslucent(background))).toEqual([]);
		expect(view).not.toMatch(/backdrop-blur|absolute inset-0/);
	});

	test('el tablero de tiempo de pantalla es un applet: su raíz es la de AppletPopover', () => {
		// vasak-desktop#150. La superficie que se mide arriba («el menú, los
		// applets y la bandeja») es la suya; acá se exige que no dibuje otra
		// raíz propia ni un bloque opaco encima del escritorio.
		const view = template('src/views/applets/ScreenTimeAppletView.vue');
		expect(view.slice('<template>'.length).trimStart().startsWith('<AppletPopover applet="screen-time"')).toBe(true);
		const opaque = backgroundsOf(view).filter((background) => !isTranslucent(background));
		expect(opaque).toEqual([]);
		expect(view).not.toMatch(/backdrop-blur/);
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

describe('la vista radial del Bluetooth y de la red (#132)', () => {
	test('va en la superficie del applet y su raíz no pinta un fondo encima', () => {
		for (const view of ['BluetoothAppletView', 'NetworkAppletView']) {
			expect(template(`src/views/applets/${view}.vue`)).toMatch(/<AppletPopover applet="[a-z]+">\s*<ConnectionsArea/);
		}
		const root = template('src/components/areas/connections/ConnectionsArea.vue').match(
			/<div class="(@container[^"]*)"/
		)?.[1];
		expect(root).toBeDefined();
		expect(backgroundsOf(root as string)).toEqual([]);
		expect(root).not.toMatch(/backdrop-blur/);
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

	test('rechaza que el marco de los widgets pierda el desenfoque', () => {
		expect(surfaceProblems(['bg-ui-shell shadow-surface-m'], 'blur')).toEqual(['sin backdrop-blur-md']);
		expect(surfaceProblems(['bg-ui-shell shadow-surface-m backdrop-blur-md'], 'blur')).toEqual([]);
	});

	test('y que otra superficie lo gane, con cualquier intensidad', () => {
		expect(surfaceProblems(['bg-ui-shell backdrop-blur-md'])).toEqual(['con backdrop-blur']);
		expect(surfaceProblems(['bg-ui-shell backdrop-blur'])).toEqual(['con backdrop-blur']);
		expect(surfaceProblems(['bg-ui-shell backdrop-blur-xl'])).toEqual(['con backdrop-blur']);
	});

	test('y deja pasar los translúcidos', () => {
		expect(surfaceProblems(['bg-ui-shell shadow-surface-l', 'bg-ui-surface/70', 'bg-ui-bg/80 hover:bg-ui-hover'])).toEqual([]);
	});
});
