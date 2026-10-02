/**
 * La guardia del diseño del taller, adaptada al escritorio.
 *
 * Es la misma de `vue-libvasak` (`tests/tokens-exist.test.ts`), mirando `src/`:
 *
 * - **Que exista lo que se usa.** Tailwind 4 arma las utilidades desde las
 *   variables del `@theme`, y una clase que nombra un token no declarado no
 *   emite ninguna regla ni avisa: el elemento se queda con lo que herede y se
 *   ve «casi bien», que es por qué estas clases muertas sobreviven años (el
 *   escritorio tuvo 15; la peor, `text-vsk-text` en 14 lugares). Los tokens se
 *   leen de los dos CSS que los declaran —el `main.css` de acá y el
 *   `tokens.css` de la librería— y no de una lista escrita en la prueba, que se
 *   queda vieja en cuanto alguien agrega un color y desde ahí miente en la
 *   dirección cómoda.
 * - **Los colores salen del esquema.** Ni hexadecimales, ni `rgb()`, ni la
 *   paleta de Tailwind, ni siquiera en una sombra: todo color termina en una
 *   `--use-*`, que pisa el config-manager con el esquema que eligió la persona.
 *   La única excepción es el **piso** de `main.css`: los valores de las
 *   variables del esquema que rigen hasta que el config-manager escribe los de
 *   verdad. Ahí un hexadecimal es la definición del esquema por omisión, no un
 *   color dibujado, y sólo puede aparecer como valor de una variable.
 * - **Los radios salen del radio del usuario.** La escala `rounded-corner-*`
 *   de `tokens.css`, derivada de `--corner-radius`; ni `rounded-md` ni
 *   `rounded-full` ni `rounded` a secas, ni un `border-radius` escrito a mano,
 *   ni los alias viejos (`rounded-corner`, `rounded-corner-sm`), que valen lo
 *   mismo que `-m` y `-xs` pero esconden cuál de los pasos es.
 * - **Los iconos salen del tema del sistema** (`ThemeIcon`): ni SVG en línea,
 *   ni imágenes propias, ni fuentes de iconos.
 * - **Ningún punto de corte de la pantalla**: un componente no sabe en qué
 *   ventana está, y en WebKitGTK ni `matchMedia` ni `resize` avisan. Lo que
 *   cambia con el ancho va con consultas de contenedor o `ResizeObserver`.
 * - **La forma de Once UI**: ni las sombras de Tailwind (que además traen su
 *   negro fijo), ni desenfoque detrás (una superficie de capa no ve el
 *   escritorio: el `backdrop-blur` cuesta y no muestra nada), ni escalas, giros
 *   o desplazamientos al pasar o al apretar, ni duraciones fuera de 100, 150,
 *   200 y 300 ms.
 *
 * Nació acá (vasak-desktop#144), pasó a la plantilla `vapp` y a vasak-terminal
 * con los huecos cerrados —el `//` adentro de un texto, las sombras `xs`, las
 * hojas de estilo, el piso del esquema—, y vuelve con ellos (vue-libvasak#74).
 */

import { describe, expect, test } from 'bun:test';
import { Glob } from 'bun';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('../', import.meta.url));
const SOURCE = `${ROOT}src/`;
const APP_CSS = `${SOURCE}assets/main.css`;
const LIBRARY_TOKENS = `${ROOT}node_modules/@vasakgroup/vue-libvasak/dist/tokens.css`;

/**
 * La bandeja del sistema dibuja el icono que **manda la otra aplicación**
 * (`IconPixmap` de StatusNotifierItem) cuando no trae un nombre del tema. No es
 * un icono propio: es el de un tercero, que el tema no puede tener. Es la
 * excepción de la §5 del inventario de vue-libvasak#74, y la única.
 */
const THIRD_PARTY_PIXMAP = ['components/buttons/TrayPixmap.vue'];

/**
 * Sin comentarios: lo que se explica no es lo que se dibuja.
 *
 * Se recorre a mano y no con un reemplazo de expresiones regulares: sacar un
 * comentario con `replace` puede juntar los pedazos de otro, y un recorrido
 * que corta por delimitadores no tiene ese problema.
 */
function stripComments(text: string): string {
	const pairs: Array<[string, string]> = [
		['<!--', '-->'],
		['/*', '*/'],
	];
	let out = '';
	let index = 0;
	while (index < text.length) {
		const char = text[index] as string;
		// Un texto entre comillas se copia entero: un `//` o un `/*` adentro
		// (`title="a // b"`) no abre ningún comentario, y cortarlo ahí se
		// llevaba puesto lo que venía después en la misma línea.
		if (char === '"' || char === '`') {
			let end = index + 1;
			while (end < text.length && text[end] !== char) end += text[end] === '\\' ? 2 : 1;
			out += text.slice(index, end + 1);
			index = end + 1;
			continue;
		}
		const pair = pairs.find(([open]) => text.startsWith(open, index));
		if (pair) {
			const end = text.indexOf(pair[1], index + pair[0].length);
			index = end === -1 ? text.length : end + pair[1].length;
			continue;
		}
		// `//` de línea, salvo dentro de una dirección (`https://`, con su
		// esquema delante: un `clave: // nota` sí es comentario) o de un texto
		// con comilla simple, que no se sigue: en la prosa de una plantilla es
		// un apóstrofo y no abre nada.
		const previous = text[index - 1] ?? '';
		const isUrl = /[a-z][a-z0-9+.-]*:$/i.test(text.slice(Math.max(0, index - 32), index));
		if (text.startsWith('//', index) && !isUrl && previous !== "'") {
			const end = text.indexOf('\n', index);
			index = end === -1 ? text.length : end;
			continue;
		}
		out += char;
		index += 1;
	}
	return out;
}

async function read(path: string): Promise<string> {
	return stripComments(await Bun.file(path).text());
}

/** Los fuentes de `src/`, sin las pruebas: una prueba nombra lo prohibido a propósito. */
function sources(pattern: string): string[] {
	return [...new Glob(pattern).scanSync(SOURCE)].filter((file) => !file.endsWith('.test.ts')).sort();
}

/** Los nombres que declaran los dos CSS, por espacio. */
async function declaredTokens() {
	const css = (await read(APP_CSS)) + (await read(LIBRARY_TOKENS));
	const names = [...css.matchAll(/--([a-z0-9-]+)\s*:/g)].map((m) => m[1] as string);
	const strip = (prefix: string) =>
		new Set(
			names
				.filter((n) => n.startsWith(prefix) && !n.includes('--'))
				.map((n) => n.slice(prefix.length))
		);
	return {
		colors: strip('color-'),
		radius: strip('radius-'),
		shadow: strip('shadow-'),
		text: strip('text-'),
		ease: strip('ease-'),
	};
}

/** Cada aparición de `regex` en cada archivo, como `archivo: coincidencia`. */
async function findAll(
	files: string[],
	regex: RegExp,
	keep: (match: RegExpMatchArray) => boolean = () => true
): Promise<string[]> {
	const found: string[] = [];
	for (const file of files) {
		const text = await read(SOURCE + file);
		for (const match of text.matchAll(regex)) {
			if (keep(match)) found.push(`${file}: ${match[0].trim()}`);
		}
	}
	return found;
}

const END = '(?![a-z0-9-])';
const VARIANTS = '(?:[a-z0-9@[\\]-]+:)*';

const COLOR_CLASS = new RegExp(
	`(?<![\\w-])${VARIANTS}(?:bg|text|border(?:-[trblxy])?|ring|outline|from|via|to|fill|stroke|divide|placeholder|decoration|accent|caret|shadow)-((?:ui|tx|status|vsk)-[a-z0-9]+(?:-[a-z0-9]+)*|primary|secondary)(?:\\/\\d+)?${END}`,
	'g'
);

const LITERAL_COLOR =
	/#[0-9a-fA-F]{3,8}(?![\w-])|(?<![a-zA-Z])(?:rgba?|hsla?|oklch|oklab|lab|lch|hwb)\(|(?<![\w-])(?:[a-z0-9@[\]-]+:)*(?:bg|text|border|ring|outline|from|via|to|fill|stroke|shadow|divide|accent|caret|decoration|drop-shadow)-(?:white|black|(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\d{2,3})(?![\w-])/g;

const FIXED_RADIUS =
	/(?<![\w-])(?:[a-z0-9@[\]-]+:)*rounded(?:-[trblse]{1,2})?(?:-(?:none|xs|sm|md|lg|xl|2xl|3xl|4xl|full|\[[^\]]*\]))?(?![\w-])|(?<![\w-])(?:[a-z0-9@[\]-]+:)*rounded(?:-[trblse]{1,2})?-corner(?:-sm)?(?![\w-])|border-radius\s*:(?!\s*var\(--radius-)|borderRadius\s*:/g;

const EMBEDDED_ICON =
	/<svg[\s>]|data:image\/|['"][^'"\s]+\.(?:svg|png|ico|webp|gif)['"]|(?<![\w-])(?:fa[srlbd]?-[a-z0-9-]+|mdi-[a-z0-9-]+|material-icons|material-symbols(?:-[a-z]+)?)(?![\w-])/g;

const VIEWPORT =
	/(?<![\w@-])(?:max-|min-)?(?:sm|md|lg|xl|2xl)(?:\/[a-z]+)?:[a-z-]|(?<![\w@-])(?:max|min)-\[[^\]]+\]:|matchMedia\(\s*[`'"]\(?\s*(?:max|min)-(?:width|height)|@media\b[^{}]*\(\s*(?:max|min)-(?:width|height)\b/g;

const FORBIDDEN_SHAPE: Array<[string, RegExp]> = [
	[
		'sombras de Tailwind en vez de shadow-surface-*',
		/(?<![\w-])(?:[a-z0-9@[\]-]+:)*(?:shadow(?:-(?:2xs|xs|sm|md|lg|xl|2xl|inner))?|drop-shadow(?:-(?:xs|sm|md|lg|xl|2xl|\[[^\]]*\]))?)(?![\w-])/g,
	],
	['desenfoque detrás', /backdrop-blur/g],
	[
		'escalas, giros y desplazamientos al pasar o al apretar',
		/(?<![\w-])(?:hover|active|focus|focus-visible|group-hover|group-active):-?(?:scale|rotate|translate)-/g,
	],
];

describe('lo que se usa existe', () => {
	test('los dos CSS se leyeron y declaran lo que tienen que declarar', async () => {
		// Sin esto, un CSS que no se encuentre deja los conjuntos vacíos, la
		// guardia de abajo no encuentra nada declarado… y como tampoco lo
		// compara, pasa siempre.
		const tokens = await declaredTokens();

		for (const color of ['tx-main', 'tx-muted', 'ui-surface', 'status-error', 'primary', 'ui-line', 'ui-hover', 'ui-float', 'ui-shell', 'ui-focus', 'ui-selected-accent']) {
			expect(tokens.colors, `falta --color-${color}`).toContain(color);
		}
		for (const radius of ['corner-xs', 'corner-s', 'corner-m', 'corner-l', 'corner-xl', 'corner-full', 'corner-window']) {
			expect(tokens.radius, `falta --radius-${radius}`).toContain(radius);
		}
		expect(tokens.shadow).toContain('surface-l');
		expect(tokens.text).toContain('label-m');
		expect(tokens.ease).toContain('ui-out');
	});

	test('ninguna clase nombra un color del taller que no esté declarado', async () => {
		const { colors } = await declaredTokens();

		const dead = await findAll(sources('**/*.{vue,ts}'), COLOR_CLASS, (m) => !colors.has(m[1] as string));

		expect(dead).toEqual([]);
	});

	test('ni un radio, una sombra, un rol de texto o una curva que no existan', async () => {
		const tokens = await declaredTokens();
		const families: Array<[RegExp, Set<string>]> = [
			// Cualquier nombre de radio que no sea de Tailwind tiene que estar
			// declarado: `rounded-b-window` no existía —el token es
			// `corner-window`— y el marco de la ventana quedaba con las esquinas
			// de abajo rectas sin que nada avisara.
			[new RegExp(`(?<![\\w-])${VARIANTS}rounded(?:-[trblse]{1,2})?-((?!(?:none|xs|sm|md|lg|xl|2xl|3xl|4xl|full)${END})[a-z][a-z0-9-]*)${END}`, 'g'), tokens.radius],
			[new RegExp(`(?<![\\w-])${VARIANTS}shadow-(surface[a-z0-9-]*)${END}`, 'g'), tokens.shadow],
			[new RegExp(`(?<![\\w-])${VARIANTS}text-((?:label|body|heading)-[a-z0-9]+)${END}`, 'g'), tokens.text],
			[new RegExp(`(?<![\\w-])${VARIANTS}ease-(ui[a-z0-9-]*)${END}`, 'g'), tokens.ease],
		];

		const dead: string[] = [];
		for (const [regex, declared] of families) {
			dead.push(...(await findAll(sources('**/*.{vue,ts}'), regex, (m) => !declared.has(m[1] as string))));
		}

		expect(dead).toEqual([]);
	});

	test('y los tokens que ya no existen no vuelven por su nombre viejo', async () => {
		// Los dos que había. Van por nombre además de por la regla general
		// porque el mensaje de arriba dice «no está declarado» y éste dice cuál
		// es el reemplazo, que es lo que hace falta cuando reaparecen.
		const replacements: Record<string, string> = { 'vsk-text': 'tx-main', 'ui-main': 'tx-main' };

		const used = await findAll(sources('**/*.{vue,ts}'), COLOR_CLASS);
		for (const [old, current] of Object.entries(replacements)) {
			expect(
				used.filter((hit) => hit.endsWith(`-${old}`)),
				`«${old}» no existe: va «${current}»`
			).toEqual([]);
		}
	});

	test('la guardia ve una clase muerta cuando la hay', () => {
		// Si la expresión no viera nada, las pruebas de arriba pasarían siempre.
		// `border-ui-border-strong` tiene que leerse entero, no como
		// `ui-border` —que existe— seguido de basura.
		const found = [...'hover:bg-ui-hoover border-ui-border-strong text-tx-main/60'.matchAll(COLOR_CLASS)].map(
			(m) => m[1]
		);

		expect(found).toEqual(['ui-hoover', 'ui-border-strong', 'tx-main']);
	});
});

describe('los colores salen del esquema', () => {
	test('ningún color escrito a mano en los componentes', async () => {
		expect(await findAll(sources('**/*.{vue,ts}'), LITERAL_COLOR)).toEqual([]);
	});

	test('ni en las hojas de estilo, salvo el piso del esquema', async () => {
		// Un hexadecimal en `main.css` sólo vale como valor de una variable: es
		// el esquema por omisión, el que rige hasta que escribe el
		// config-manager. Cualquier otro uso —un `color:`, una sombra, un
		// `@apply bg-white`— es un color dibujado a mano.
		const found: string[] = [];
		for (const file of sources('**/*.css')) {
			for (const line of (await read(SOURCE + file)).split('\n')) {
				if (!line.match(LITERAL_COLOR)) continue;
				// Sólo en `main.css`: otra hoja que declare `--algo: #fff` y
				// después lo use es un color dibujado a mano con un paso más.
				if (`${SOURCE}${file}` === APP_CSS && /^\s*--[a-z0-9-]+\s*:\s*#[0-9a-fA-F]{3,8}\s*;\s*$/.test(line)) continue;
				found.push(`${file}: ${line.trim()}`);
			}
		}

		expect(found).toEqual([]);
	});

	test('el color del texto por omisión no pisa las utilidades', async () => {
		// `body * { @apply text-tx-main }` fuera de toda capa le ganaba a cada
		// `text-*` de Tailwind, que vive en `@layer utilities`: el texto sobre
		// el primario salía con el color principal (1,4:1 en oscuro) y ningún
		// `text-tx-muted` se veía (vasak-desktop#144). El color por omisión va
		// heredado y en la capa `base`. La transición de colores sí puede ir
		// en `body *`: no es un color.
		const css = await read(APP_CSS);
		expect(css).not.toMatch(/body\s*\*\s*\{[^}]*(?:(?<![\w-])color\s*:|text-tx)/);
		expect(css).toMatch(/@layer base\s*\{\s*body\s*\{\s*color:\s*var\(--color-tx-main\);/);
	});

	test('y no vuelve la clase `.background`', async () => {
		// La librería 2.0 dejó de leerla: sus tarjetas van en
		// `bg-ui-surface/70`. Una aplicación que la siga declarando no cambia
		// nada en la librería y sí invita a usarla en lo propio, con el fondo de
		// la ventana encima de la ventana (memoria `tokens-de-fondo`).
		expect(await read(APP_CSS)).not.toMatch(/(?<![\w-])\.background\b/);
	});

	test('respeta a quien pidió menos movimiento', async () => {
		// `tokens.css` no lo trae: lo pone cada aplicación. Sin esto, la
		// transición de colores de `body *` y las entradas de los componentes
		// siguen moviéndose para quien lo pidió (WCAG 2.3.3).
		const css = await read(APP_CSS);
		const block = css.slice(css.indexOf('@media (prefers-reduced-motion: reduce)'));
		expect(block.length, 'falta el bloque de prefers-reduced-motion').toBeGreaterThan(0);
		expect(block).toMatch(/transition-duration:\s*0\.01ms\s*!important/);
		expect(block).toMatch(/animation-iteration-count:\s*1\s*!important/);
	});

	test('ni las copias de lo que trae `tokens.css`', async () => {
		// Los radios y el piso del foco viven en la librería. Una copia acá,
		// por venir después, le gana: así el foco volvía al primario (2,64:1
		// contra el fondo claro, debajo del 3:1 que pide un indicador).
		const css = await read(APP_CSS);
		expect(css).toContain('@import "@vasakgroup/vue-libvasak/tokens.css"');
		expect(css.indexOf('@import "tailwindcss"')).toBeLessThan(css.indexOf('@import "@vasakgroup/vue-libvasak/tokens.css"'));
		expect(css).not.toMatch(/--radius-[a-z0-9-]+\s*:/);
		expect(css).not.toMatch(/:focus-visible/);
	});

	test('la guardia de colores ve un color cuando lo hay', () => {
		const sample = 'color: #dd7878; box-shadow: 0 0 1px rgb(0 0 0 / .1); hover:bg-white text-gray-500';
		expect([...sample.matchAll(LITERAL_COLOR)]).toHaveLength(4);
		// También pegado a un guion bajo, como en un valor arbitrario de
		// Tailwind: `\b` no ve el borde entre `_` y `rgba`.
		expect([...'drop-shadow-[0_0_6px_rgba(59,130,246,0.5)]'.matchAll(LITERAL_COLOR)]).toHaveLength(1);
		// Y un comentario no es un color.
		expect(stripComments('/* #dd7878 */ <!-- rgb(1 2 3) -->')).not.toMatch(LITERAL_COLOR);
		// Pero un `//` adentro de un texto no se come lo que sigue. (Sin la
		// `g`: con ella, `toMatch` arranca donde terminó la búsqueda anterior.)
		const once = new RegExp(LITERAL_COLOR.source);
		expect(stripComments('<div title="a // b" class="bg-white rounded-md"></div>')).toMatch(once);
		expect(stripComments('<div title="a /* b" class="bg-white"></div> */')).toMatch(once);
		// Y una dirección no es un comentario, pero `clave: // nota` sí.
		expect(stripComments("src: url(https://x/bg-white)")).toMatch(once);
		expect(stripComments('gap: // bg-white')).not.toMatch(once);
	});
});

describe('los radios salen del radio que eligió la persona', () => {
	test('ningún radio fijo ni alias viejo', async () => {
		expect(await findAll(sources('**/*.{vue,ts,css}'), FIXED_RADIUS)).toEqual([]);
	});

	test('la guardia de radios ve un radio fijo cuando lo hay', () => {
		const sample =
			'rounded-md rounded-full rounded hover:rounded-lg rounded-[6px] rounded-corner rounded-corner-sm border-radius: 4px';
		expect([...sample.matchAll(FIXED_RADIUS)]).toHaveLength(8);
		expect([...'rounded-corner-m rounded-t-corner-xl border-radius: var(--radius-corner-xs)'.matchAll(FIXED_RADIUS)]).toHaveLength(0);
	});
});

describe('los iconos salen del tema del sistema', () => {
	test('ningún icono propio', async () => {
		const files = sources('**/*.{vue,ts}').filter((file) => !THIRD_PARTY_PIXMAP.includes(file));
		expect(await findAll(files, EMBEDDED_ICON)).toEqual([]);
	});

	test('la excepción de la bandeja sigue siendo sólo el mapa de bits del tercero', async () => {
		// Si el archivo deja de existir o se llena de otras cosas, la excepción
		// tapa más de lo que tiene que tapar.
		for (const file of THIRD_PARTY_PIXMAP) {
			expect(await findAll([file], EMBEDDED_ICON)).toEqual([`${file}: data:image/`]);
		}
	});

	test('ni imágenes de iconos en el repositorio', () => {
		// El escritorio guardaba el logo y dos fondos de la plantilla en
		// `src/assets/`, que nada usaba. El icono de la aplicación es el de
		// `src-tauri/icons/`, que es otra cosa: lo que instala el paquete en el
		// tema.
		const images = ['src/', 'public/'].filter((dir) => existsSync(ROOT + dir)).flatMap((dir) =>
			[...new Glob('**/*.{svg,png,ico,webp,gif}').scanSync({ cwd: ROOT + dir, onlyFiles: true })].map((f) => dir + f)
		);
		expect(images).toEqual([]);
	});

	test('la guardia de iconos ve uno cuando lo hay', () => {
		expect([...'<svg viewBox="0 0 1 1"> src="./x.png" mdi-home'.matchAll(EMBEDDED_ICON)]).toHaveLength(3);
	});
});

describe('ningún punto de corte de la pantalla', () => {
	test('un componente no sabe en qué ventana está', async () => {
		// También en los `<style>` y en las hojas: un `@media (min-width)` es
		// el mismo punto de corte escrito a mano.
		expect(await findAll(sources('**/*.{vue,ts,css}'), VIEWPORT)).toEqual([]);
	});

	test('la guardia deja pasar los de contenedor', () => {
		expect([...'md:w-72 sm:flex-row'.matchAll(VIEWPORT)]).toHaveLength(2);
		expect([...'@media (min-width: 640px) {} @media screen and (max-height: 20rem)'.matchAll(VIEWPORT)]).toHaveLength(2);
		expect([...'@media (prefers-reduced-motion: reduce)'.matchAll(VIEWPORT)]).toHaveLength(0);
		expect([...'@sm:flex-row @md:w-72 @min-[40rem]:grid-cols-2'.matchAll(VIEWPORT)]).toHaveLength(0);
	});
});

describe('lo que la forma de Once UI deja afuera', () => {
	for (const [what, regex] of FORBIDDEN_SHAPE) {
		test(`sin ${what}`, async () => {
			// También en las hojas: un `@apply shadow-lg` es la misma sombra.
			expect(await findAll(sources('**/*.{vue,ts,css}'), regex)).toEqual([]);
		});
	}

	test('las duraciones son 100, 150, 200 o 300', async () => {
		const found = await findAll(
			sources('**/*.{vue,ts,css}'),
			/(?<![\w-])(?:[a-z0-9@[\]-]+:)*duration-(\d+)(?![\w-])/g,
			(m) => !['100', '150', '200', '300'].includes(m[1] as string)
		);

		expect(found).toEqual([]);
	});

	test('la guardia ve lo prohibido cuando lo hay', () => {
		const sample = 'shadow-lg shadow-xs shadow-2xs drop-shadow-[0_0_6px] hover:scale-110 backdrop-blur-md shadow-surface-l';
		const hits = FORBIDDEN_SHAPE.flatMap(([what, regex]) => [...sample.matchAll(regex)].map(() => what));

		expect(hits).toEqual([
			'sombras de Tailwind en vez de shadow-surface-*',
			'sombras de Tailwind en vez de shadow-surface-*',
			'sombras de Tailwind en vez de shadow-surface-*',
			'sombras de Tailwind en vez de shadow-surface-*',
			'desenfoque detrás',
			'escalas, giros y desplazamientos al pasar o al apretar',
		]);
	});
});

describe('el piso del esquema se lee', () => {
	/**
	 * Los valores de `:root` rigen hasta que el config-manager escribe los del
	 * esquema de la persona, y son los que quedan si esa lectura falla. Este
	 * diálogo llegó a tener `--text-on-primary-dark: #cdd6f4`: «Permitir» salía
	 * lavanda sobre rosa, 1,4:1, hasta que cargaba la configuración.
	 */
	function luminance(hex: string): number {
		const value = hex.replace('#', '');
		const full = value.length === 3 ? [...value].map((c) => c + c).join('') : value;
		const [r, g, b] = [0, 2, 4].map((i) => {
			const channel = Number.parseInt(full.slice(i, i + 2), 16) / 255;
			return channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
		});
		return 0.2126 * (r as number) + 0.7152 * (g as number) + 0.0722 * (b as number);
	}

	function contrast(a: string, b: string): number {
		const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
		return (light + 0.05) / (dark + 0.05);
	}

	async function floor(): Promise<Record<string, string>> {
		const css = await read(APP_CSS);
		const root = css.slice(css.indexOf(':root {'), css.indexOf('}', css.indexOf(':root {')));
		return Object.fromEntries([...root.matchAll(/--([a-z0-9-]+)\s*:\s*(#[0-9a-fA-F]{3,6})\s*;/g)].map((m) => [m[1], m[2]]));
	}

	for (const mode of ['', '-dark']) {
		test(`el texto llega a 4,5:1 sobre su fondo${mode ? ' en oscuro' : ' en claro'}`, async () => {
			const colors = await floor();
			const pairs: Array<[string, string]> = [
				['text-on-primary', 'primary'],
				['text-main', 'ui-background'],
				['text-muted', 'ui-background'],
				['text-main', 'ui-surface'],
			];
			for (const [text, background] of pairs) {
				const fg = colors[text + mode];
				const bg = colors[background + mode];
				expect(fg, `falta --${text}${mode}`).toBeDefined();
				expect(bg, `falta --${background}${mode}`).toBeDefined();
				expect(contrast(fg as string, bg as string), `--${text}${mode} sobre --${background}${mode}`).toBeGreaterThanOrEqual(4.5);
			}
		});
	}

	test('y el contorno de un control, 3:1 contra el fondo', async () => {
		const colors = await floor();
		for (const mode of ['', '-dark']) {
			expect(contrast(colors[`ui-border-strong${mode}`] as string, colors[`ui-background${mode}`] as string)).toBeGreaterThanOrEqual(3);
		}
	});
});
