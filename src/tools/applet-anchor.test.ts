import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
	anchorFromQuery,
	anchorOf,
	appletTransformOrigin,
	fullFromQuery,
	insetFromQuery,
	insetStyle,
	NO_INSET,
	toFull,
	toInset,
} from './applet-anchor';
import { applyAppletChanged, openAppletForTests } from './composables/useOpenApplet';

const ROOT = join(import.meta.dir, '..', '..');
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

describe('de dónde crece un applet', () => {
	test('con el panel arriba, desde el botón y el borde de arriba', () => {
		expect(appletTransformOrigin({ side: 'top', origin: 120 })).toBe('120px 0%');
	});

	test('con el panel abajo, desde el botón y el borde de abajo', () => {
		expect(appletTransformOrigin({ side: 'bottom', origin: 120 })).toBe('120px 100%');
	});

	test('a los costados, desde el borde que toca el panel y la altura del botón', () => {
		expect(appletTransformOrigin({ side: 'left', origin: 80 })).toBe('0% 80px');
		expect(appletTransformOrigin({ side: 'right', origin: 80 })).toBe('100% 80px');
	});

	test('sin anclaje, desde el centro del borde de arriba', () => {
		// Es donde está el panel por omisión: una página abierta sin que el
		// backend dijera nada sigue creciendo desde el lado más probable.
		expect(appletTransformOrigin(null)).toBe('50% 0%');
	});

	test('un origen que no es un número no se escribe en el estilo', () => {
		// `NaNpx` no es un `transform-origin`: el navegador descarta la regla y
		// el applet crece desde el centro, que es justo lo que se vino a sacar.
		expect(appletTransformOrigin({ side: 'top', origin: Number.NaN })).toBe('50% 0%');
		expect(appletTransformOrigin({ side: 'left', origin: -5 })).toBe('0% 0px');
	});
});

describe('el anclaje que viene en la ruta', () => {
	test('se lee el lado y el origen', () => {
		expect(anchorFromQuery({ side: 'bottom', origin: '212.5' })).toEqual({
			side: 'bottom',
			origin: 212.5,
		});
	});

	test('un lado que no existe o un origen que no se entiende descartan todo', () => {
		expect(anchorFromQuery({ side: 'arriba', origin: '10' })).toBeNull();
		expect(anchorFromQuery({ side: 'top', origin: 'mucho' })).toBeNull();
		expect(anchorFromQuery({ side: 'top' })).toBeNull();
		expect(anchorFromQuery({})).toBeNull();
	});

	test('con la clave repetida, vale la primera', () => {
		expect(anchorFromQuery({ side: ['right', 'top'], origin: ['40', '0'] })).toEqual({
			side: 'right',
			origin: 40,
		});
	});
});

describe('el margen de sombra', () => {
	test('la ruta de la primera apertura lo trae en el orden de Rust', () => {
		// izquierda, derecha, arriba, abajo: el mismo orden que `Placement::inset`.
		expect(insetFromQuery({ side: 'top', origin: '23', inset: '10,24,24,24' })).toEqual({
			left: 10,
			right: 24,
			top: 24,
			bottom: 24,
		});
	});

	test('sin margen en la ruta, el applet ocupa la superficie entera', () => {
		expect(insetFromQuery({ side: 'top', origin: '10' })).toEqual(NO_INSET);
		expect(insetFromQuery({ inset: '1,2,3' })).toEqual(NO_INSET);
	});

	test('un lado que no se entiende vale cero, no `NaNpx`', () => {
		expect(insetFromQuery({ inset: '24,nada,24,-3' })).toEqual({
			left: 24,
			right: 0,
			top: 24,
			bottom: 0,
		});
		expect(toInset({ left: 24, right: '24', top: null })).toEqual({
			left: 24,
			right: 0,
			top: 0,
			bottom: 0,
		});
		expect(toInset(undefined)).toEqual(NO_INSET);
	});

	test('se escribe como la posición del applet dentro de la superficie', () => {
		expect(insetStyle({ left: 10, right: 24, top: 24, bottom: 24 })).toEqual({
			left: '10px',
			right: '24px',
			top: '24px',
			bottom: '24px',
		});
	});
});

describe('el overlay a pantalla completa', () => {
	test('la ruta lo pide con `full=1`, y sin eso no es overlay', () => {
		expect(fullFromQuery({ side: 'top', origin: '0', inset: '0,0,0,0', full: '1' })).toBe(true);
		expect(fullFromQuery({ side: 'top', origin: '0', inset: '0,0,0,0' })).toBe(false);
		// El applet anclado de siempre: nunca es overlay.
		expect(fullFromQuery({ side: 'top', origin: '23' })).toBe(false);
	});

	test('el evento lo trae como booleano, y cualquier otra cosa es `false`', () => {
		expect(toFull(true)).toBe(true);
		expect(toFull('1')).toBe(true);
		expect(toFull('true')).toBe(true);
		expect(toFull(false)).toBe(false);
		expect(toFull(undefined)).toBe(false);
		expect(toFull('0')).toBe(false);
		expect(toFull(0)).toBe(false);
	});

	test('si viene repetido en la ruta, vale el primero', () => {
		expect(fullFromQuery({ full: ['1', '0'] })).toBe(true);
	});
});

describe('el rectángulo del botón', () => {
	const rect = { x: 10, y: 2, width: 30, height: 34 };
	const element = { getBoundingClientRect: () => ({ ...rect, top: 2, toJSON: () => null }) };

	test('se mide del elemento, sin lo que no viaja', () => {
		expect(anchorOf(element)).toEqual(rect);
	});

	test('y también del componente, que es lo que da un `ref` sobre uno de la librería', () => {
		expect(anchorOf({ $el: element })).toEqual(rect);
	});

	test('sin nada que medir no hay rectángulo, y el backend centra el applet', () => {
		expect(anchorOf(null)).toBeUndefined();
		expect(anchorOf(undefined)).toBeUndefined();
		expect(anchorOf({ $el: null })).toBeUndefined();
		expect(anchorOf({})).toBeUndefined();
	});
});

describe('qué applet está abierto', () => {
	test('lo dice el backend, abra o cierre quien sea', () => {
		applyAppletChanged({ applet: 'audio' });
		expect(openAppletForTests.value).toBe('audio');

		applyAppletChanged({ applet: null });
		expect(openAppletForTests.value).toBeNull();

		applyAppletChanged(undefined);
		expect(openAppletForTests.value).toBeNull();
	});
});

describe('AppletPopover', () => {
	const popover = read('src/components/layouts/AppletPopover.vue');

	test('el origen de la animación sale del anclaje', () => {
		expect(popover).toContain('appletTransformOrigin(anchor.value)');
		expect(popover).toContain(':style="placement"');
		expect(popover).toMatch(/transformOrigin: transformOrigin\.value/);
	});

	test('el applet se dibuja dentro del margen de sombra', () => {
		// La superficie es más grande que el applet: sin esto el applet llenaría
		// el margen y quedaría corrido 24 píxeles hacia afuera.
		expect(popover).toMatch(/\.\.\.insetStyle\(inset\.value\)/);
		expect(popover).toContain('insetFromQuery(route.query)');
		expect(popover).toContain('inset.value = toInset(payload.inset)');
		expect(popover).toMatch(/'applet-popover absolute /);
	});

	test('la sombra sale de un token, nunca de un color escrito', () => {
		// En el applet anclado; el overlay a pantalla completa no lleva sombra.
		expect(popover).toContain('shadow-surface-l');
		expect(popover).not.toMatch(/rgba?\(|#[0-9a-f]{3,8}\b/i);
	});

	test('y el token existe: el escritorio importa la forma de la librería', () => {
		// Sin `tokens.css` la clase no emite nada y el applet queda sin sombra,
		// sin que nada avise. Era lo que pasaba con la 1.x: la sombra estaba
		// pedida y el token no existía.
		const css = read('src/assets/main.css');
		const tokens = read('node_modules/@vasakgroup/vue-libvasak/dist/tokens.css');
		expect(css).toMatch(
			/@import "tailwindcss";\s[\s\S]*@import "@vasakgroup\/vue-libvasak\/tokens\.css";/
		);
		expect(tokens).toMatch(/--shadow-surface-l:/);
	});

	test('translúcido, del canto fino y con el radio de lo que flota', () => {
		const base = popover.match(/'applet-popover [^']*'/)?.[0] ?? '';
		// `ui-shell` y no `ui-float`: el desenfoque lo pone Wayfire detrás, y
		// una superficie opaca lo tapa (corrección del 02/10/2026). Es del fondo,
		// que comparten el applet anclado y el overlay.
		expect(base).toContain('bg-ui-shell');
		expect(base).not.toContain('bg-ui-float');
		expect(popover).not.toContain('backdrop-blur');
		// El canto fino y el radio son del applet anclado, no del overlay: viven
		// en la rama que elige `full`.
		expect(popover).toMatch(/full\s*\?[\s\S]*rounded-corner-xl border border-ui-line/);
	});

	test('el overlay a pantalla completa no lleva borde, radio ni sombra', () => {
		// No hay afuera donde se vean (vasak-desktop#210): la rama de `full` es la
		// que se queda sin la forma de lo que flota.
		expect(popover).toContain('full.value = toFull(payload.full)');
		expect(popover).toContain('fullFromQuery(route.query)');
		expect(popover).toMatch(/full\s*\?\s*'applet-popover-full'/);
	});

	test('el overlay entra con un fundido, sin crecer desde el botón', () => {
		expect(popover).toMatch(/\.applet-popover-full\.applet-popover-enter\s*{[\s\S]*fade-in/);
		expect(popover).toMatch(/\.applet-popover-full\.applet-popover-leave\s*{[\s\S]*fade-out/);
	});

	test('entra con opacidad y escala desde 0.96, en 200 ms y frenando', () => {
		expect(popover).toMatch(/applet-popover-in[\s\S]*scale\(0\.96\)/);
		expect(popover).toContain('animation: applet-popover-in 200ms var(--ease-ui-out)');
	});

	test('sin movimiento, sólo opacidad', () => {
		const reduced = popover.slice(popover.indexOf('@media (prefers-reduced-motion: reduce)'));
		expect(reduced).toContain('applet-popover-fade-in');
		expect(reduced).toContain('applet-popover-fade-out');
		expect(reduced).not.toContain('scale');
	});

	test('la raíz de la plantilla es un solo elemento', () => {
		// Un comentario antes de la raíz la vuelve fragmento y las clases que se
		// le pasan desde afuera dejan de caer en algún lado.
		const template = popover.slice(popover.indexOf('<template>') + '<template>'.length).trimStart();
		expect(template.startsWith('<div')).toBe(true);
	});

	test('los applets usan el contenedor nuevo, y el viejo no existe', () => {
		for (const view of [
			'AudioAppletView',
			'BluetoothAppletView',
			'MusicAppletView',
			'NetworkAppletView',
			'PrivacyAppletView',
			'TrayPopupView',
			'TwingateAppletView',
		]) {
			const source = read(`src/views/applets/${view}.vue`);
			expect(source).toContain('<AppletPopover applet="');
			expect(source).not.toContain('AppletFrame');
		}
	});
});

describe('la interfaz y el backend nombran los mismos applets', () => {
	/**
	 * El panel pide un applet por su nombre y el backend lo busca en su tabla.
	 * Si los dos se separan, el botón no abre nada y ninguna compilación lo
	 * dice: Tauri devuelve un error que sólo se ve en el registro.
	 */
	const rust = read('src-tauri/src/windows_apps/anchored_applet.rs');
	const service = read('src/services/window.service.ts');
	const routes = read('src/routes/index.ts');

	const rustApplets = [...rust.matchAll(/id: "([a-z-]+)",\s*route: "([a-z-]+)"/g)].map(
		([, id, route]) => ({ id, route })
	);
	const declared = service
		.slice(service.indexOf('export type AppletId ='))
		.split(';')[0]
		.match(/'([a-z-]+)'/g)
		?.map((quoted) => quoted.slice(1, -1));

	test('los mismos nombres', () => {
		expect(rustApplets.length).toBeGreaterThan(0);
		expect(declared?.sort()).toEqual(rustApplets.map(({ id }) => id).sort());
	});

	test('y cada ruta del backend existe en la interfaz', () => {
		for (const { route } of rustApplets) {
			expect(routes).toContain(`path: '${route}'`);
		}
	});
});
