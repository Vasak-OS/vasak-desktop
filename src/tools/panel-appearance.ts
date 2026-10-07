/**
 * El aspecto del panel: su tipo, su densidad, su animación y su tamaño.
 *
 * Todo sale de la sección `panel` de `~/.config/vasak/vasak.conf`, la misma que
 * ya dice de qué lado va (`panel-position.ts`) y qué indicadores muestra. Se lee
 * tolerante, igual que la posición: un valor que no sea de los que se conocen
 * —o que no diga nada— cae al de fábrica, porque el archivo se edita a mano y un
 * `"isla"` mal escrito no puede dejar el panel sin dibujar.
 *
 * El tipo y la densidad sólo cambian **cómo** se dibuja la barra; la posición y
 * los indicadores siguen mandando qué hay y de qué lado. Por omisión todo queda
 * como el panel en píldoras de vasak-desktop#151: `pills` + `distributed`, que
 * es exactamente lo que había antes de esta pantalla —se prueba con una
 * aserción contra `BAR_CLASSES` para que no haya regresión visual—.
 *
 * Vive aparte de las vistas para poder probarlo sin montar Vue, igual que
 * `panel-position.ts`.
 */

import { BAR_CLASSES, type PanelPosition } from '@/tools/panel-position';

/** El token del archivo para una clave de la sección `panel`. */
function panelKey(config: unknown, key: string): unknown {
	const section =
		config && typeof config === 'object'
			? ((config as Record<string, unknown>).panel as Record<string, unknown> | undefined)
			: undefined;
	return section?.[key];
}

// ── Tipo de panel ───────────────────────────────────────────────────────────

/**
 * Los cuatro tipos de barra:
 *
 * - `pills`: píldoras sueltas sobre el escritorio, sin fondo (lo de hoy).
 * - `floating`: una barra redondeada despegada de los bordes.
 * - `bar`: de borde a borde, sin redondear (al estilo Windows).
 * - `dock`: pegada al borde dominante y redondeada sólo del lado de adentro.
 */
export const PANEL_STYLES = ['pills', 'floating', 'bar', 'dock'] as const;

export type PanelStyle = (typeof PANEL_STYLES)[number];

/** Píldoras: es lo que trajo #151 y lo que la gente ya conoce. */
export const DEFAULT_PANEL_STYLE: PanelStyle = 'pills';

export function isPanelStyle(value: unknown): value is PanelStyle {
	return typeof value === 'string' && (PANEL_STYLES as readonly string[]).includes(value);
}

export function panelStyle(config: unknown): PanelStyle {
	const value = panelKey(config, 'style');
	return isPanelStyle(value) ? value : DEFAULT_PANEL_STYLE;
}

/** Los tres tipos que dibujan una superficie continua detrás de las píldoras. */
export function hasSurface(style: PanelStyle): boolean {
	return style !== 'pills';
}

// ── Densidad ──────────────────────────────────────────────────────────────

/**
 * Cómo se reparte lo de adentro a lo largo de la barra:
 *
 * - `distributed`: los extremos empujan hacia afuera y el centro va al medio,
 *   ocupando el largo entero (lo de hoy).
 * - `compact`: todo se junta y se centra, y la barra se encoge a su contenido.
 *
 * Es otra cosa que la densidad de `panel-density.ts`, que mide el largo de la
 * barra para plegar textos: aquélla la decide el tamaño de la pantalla, ésta la
 * elige la persona.
 */
export const PANEL_LAYOUTS = ['distributed', 'compact'] as const;

export type PanelLayout = (typeof PANEL_LAYOUTS)[number];

/** Distribuido, que es como estuvo la barra siempre. */
export const DEFAULT_PANEL_LAYOUT: PanelLayout = 'distributed';

export function isPanelLayout(value: unknown): value is PanelLayout {
	return typeof value === 'string' && (PANEL_LAYOUTS as readonly string[]).includes(value);
}

export function panelLayout(config: unknown): PanelLayout {
	const value = panelKey(config, 'layout');
	return isPanelLayout(value) ? value : DEFAULT_PANEL_LAYOUT;
}

// ── Animación ───────────────────────────────────────────────────────────────

/**
 * El movimiento de las píldoras. Las curvas viven en `main.css`; acá sólo se
 * elige cuál. `off` no pone ninguna clase, así que una barra quieta no paga
 * nada. Todas respetan a quien pidió menos movimiento: el bloque de
 * `prefers-reduced-motion` de `main.css` corta la iteración a una sola.
 */
export const PANEL_ANIMATIONS = ['off', 'stream', 'wave', 'sweep', 'reactor', 'beat'] as const;

export type PanelAnimation = (typeof PANEL_ANIMATIONS)[number];

/** Apagada: el panel no se mueve hasta que la persona lo pide. */
export const DEFAULT_PANEL_ANIMATION: PanelAnimation = 'off';

export function isPanelAnimation(value: unknown): value is PanelAnimation {
	return typeof value === 'string' && (PANEL_ANIMATIONS as readonly string[]).includes(value);
}

export function panelAnimation(config: unknown): PanelAnimation {
	const value = panelKey(config, 'animation');
	return isPanelAnimation(value) ? value : DEFAULT_PANEL_ANIMATION;
}

/** La clase que enciende cada animación. `off` no pone ninguna. */
export const PANEL_ANIMATION_CLASS: Record<PanelAnimation, string> = {
	off: '',
	stream: 'panel-anim-stream',
	wave: 'panel-anim-wave',
	sweep: 'panel-anim-sweep',
	reactor: 'panel-anim-reactor',
	beat: 'panel-anim-beat',
};

export function panelAnimationClass(animation: PanelAnimation): string {
	return PANEL_ANIMATION_CLASS[animation];
}

// ── Tamaño ──────────────────────────────────────────────────────────────────

/** Entre el 80 y el 120 %: más abajo no se lee, más arriba no entra en la franja. */
export const MIN_PANEL_SIZE = 80;
export const MAX_PANEL_SIZE = 120;
export const DEFAULT_PANEL_SIZE = 100;

/**
 * El tamaño en porciento, acotado al rango que entra. El archivo se edita a
 * mano: un `150` o un `"grande"` no pueden estirar la barra fuera de su franja,
 * así que todo se recorta a [80, 120] y lo que no es un número vale 100.
 */
export function panelSize(config: unknown): number {
	const value = panelKey(config, 'size');
	if (typeof value !== 'number' || !Number.isFinite(value)) return DEFAULT_PANEL_SIZE;
	return Math.min(MAX_PANEL_SIZE, Math.max(MIN_PANEL_SIZE, Math.round(value)));
}

/** El factor que escala la barra, para la variable CSS `--panel-scale`. */
export function panelScaleStyle(size: number): Record<string, string> {
	return { '--panel-scale': String(size / 100) };
}

// ── Clases de la barra ────────────────────────────────────────────────────

/**
 * Las clases van **enteras y literales**, nunca armadas por partes.
 *
 * Tailwind 4 emite una utilidad sólo si encuentra su nombre tal cual en el
 * texto de un fuente: una clase que sólo existe concatenada en tiempo de
 * ejecución (`` `top-${gap}` ``) no se genera y no pinta nada. Por eso cada
 * posible clase aparece completa en alguno de estos mapas.
 */

/** Grosor, eje y relleno de la barra, que no cambian con el tipo ni la densidad. */
const AXIS: Record<PanelPosition, string> = {
	top: 'h-9 flex-row px-1',
	bottom: 'h-9 flex-row px-1',
	left: 'w-9 flex-col py-1 justify-items-center',
	right: 'w-9 flex-col py-1 justify-items-center',
};

/** El largo entero, para flotante y píldoras (dejan su margen contra los costados). */
const DETACHED_SPAN: Record<PanelPosition, string> = {
	top: 'w-[calc(100%-8px)] mx-1 mt-0.5',
	bottom: 'w-[calc(100%-8px)] mx-1 mb-0.5',
	left: 'h-[calc(100vh-8px)] my-1 ml-0.5',
	right: 'h-[calc(100vh-8px)] my-1 mr-0.5',
};

/** El largo entero sin margen, para barra y dock (cruzan de extremo a extremo). */
const EDGE_SPAN: Record<PanelPosition, string> = {
	top: 'w-full',
	bottom: 'w-full',
	left: 'h-screen',
	right: 'h-screen',
};

/** La grilla de tres según la densidad: los lados se encogen o se reparten. */
const TRACK_DISTRIBUTED: Record<PanelPosition, string> = {
	top: 'grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]',
	bottom: 'grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]',
	left: 'grid-rows-[minmax(0,1fr)_auto_minmax(0,1fr)]',
	right: 'grid-rows-[minmax(0,1fr)_auto_minmax(0,1fr)]',
};

const TRACK_COMPACT: Record<PanelPosition, string> = {
	top: 'grid-cols-[auto_auto_auto]',
	bottom: 'grid-cols-[auto_auto_auto]',
	left: 'grid-rows-[auto_auto_auto]',
	right: 'grid-rows-[auto_auto_auto]',
};

/** Compacto: suelto, centrado sobre el eje largo y encogido a su contenido. */
const COMPACT_CENTER: Record<PanelPosition, string> = {
	top: 'left-1/2 -translate-x-1/2 w-fit',
	bottom: 'left-1/2 -translate-x-1/2 w-fit',
	left: 'top-1/2 -translate-y-1/2 h-fit',
	right: 'top-1/2 -translate-y-1/2 h-fit',
};

/** El ancla contra el borde dominante: 2 px (píldoras/flotante) o pegada (barra/dock). */
const COMPACT_ANCHOR: Record<PanelPosition, { gap: string; flush: string }> = {
	top: { gap: 'top-0.5', flush: 'top-0' },
	bottom: { gap: 'bottom-0.5', flush: 'bottom-0' },
	left: { gap: 'left-0.5', flush: 'left-0' },
	right: { gap: 'right-0.5', flush: 'right-0' },
};

/** Píldoras y flotante dejan 2 px contra el borde; barra y dock lo tocan. */
function keepsEdgeGap(style: PanelStyle): boolean {
	return style === 'pills' || style === 'floating';
}

/**
 * Las clases de la `<nav>` según el tipo y la densidad.
 *
 * El tipo por omisión en modo distribuido es `BAR_CLASSES` tal cual, con un
 * `relative` delante —así el panel de siempre queda idéntico: la `<nav>` ya
 * llevaba `relative` en su clase fija, que acá se saca para que cada densidad
 * ponga la suya (distribuido en el flujo con `relative`, compacto suelto con
 * `absolute`) sin que dos `position` compitan—. Cada otra combinación arma su
 * propio string entero a partir de clases enteras, sin componerse sobre el
 * anterior: dos utilidades que tocan la misma propiedad (un `w-full` sobre un
 * `w-[calc(...)]`) las ordena Tailwind por su cuenta, no el orden en que se las
 * escribe.
 */
export function panelBarClasses(
	position: PanelPosition,
	style: PanelStyle,
	layout: PanelLayout
): string {
	if (style === 'pills' && layout === 'distributed') return `relative ${BAR_CLASSES[position]}`;

	const parts: string[] = [AXIS[position]];

	if (layout === 'compact') {
		const anchor = COMPACT_ANCHOR[position];
		parts.push(
			'absolute',
			COMPACT_CENTER[position],
			keepsEdgeGap(style) ? anchor.gap : anchor.flush
		);
		parts.push(TRACK_COMPACT[position]);
	} else {
		parts.push('relative');
		parts.push(keepsEdgeGap(style) ? DETACHED_SPAN[position] : EDGE_SPAN[position]);
		parts.push(TRACK_DISTRIBUTED[position]);
	}

	return parts.join(' ');
}

/**
 * Las clases de la superficie —el fondo redondeado que va detrás de las
 * píldoras en flotante, barra y dock—. En píldoras no hay superficie: devuelve
 * `''` y la vista no dibuja la capa.
 *
 * El fondo es translúcido (`bg-ui-bg/80`): el desenfoque lo pone Wayfire detrás
 * de la franja, como en el resto de las superficies del escritorio. El canto va
 * en `ui-line`, el radio en la variable del sistema (`rounded-corner-m`), y el
 * dock redondea sólo el lado de adentro —el opuesto al borde contra el que se
 * apoya—.
 */
export function panelSurfaceClass(style: PanelStyle, position: PanelPosition): string {
	if (!hasSurface(style)) return '';

	const base = 'bg-ui-bg/80 border border-ui-line';
	if (style === 'bar') return base;
	if (style === 'floating') return `${base} rounded-corner-m`;

	// Dock: el borde opuesto al que toca es el que se redondea.
	const innerRound: Record<PanelPosition, string> = {
		top: 'rounded-b-corner-m',
		bottom: 'rounded-t-corner-m',
		left: 'rounded-r-corner-m',
		right: 'rounded-l-corner-m',
	};
	return `${base} ${innerRound[position]}`;
}
