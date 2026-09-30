/**
 * De dónde crece un applet y qué rectángulo manda el panel al abrirlo.
 *
 * El backend ubica el applet colgado del botón que lo abrió
 * (`anchored_applet.rs`) y le dice a la página dos cosas: de qué lado está el
 * panel y dónde quedó el centro del botón a lo largo del applet. Con eso la
 * página arma el `transform-origin`, para que la entrada **crezca desde el
 * botón** y no desde el centro de la ventana.
 *
 * Vive aparte de `AppletPopover.vue` para poder probarlo: este repositorio no
 * tiene con qué montar componentes.
 */

import { isPanelPosition, type PanelPosition } from '@/tools/panel-position';

/** Lo que el backend le cuenta al applet al mostrarlo. */
export interface AppletAnchor {
	/** De qué lado de la pantalla está el panel. */
	side: PanelPosition;
	/**
	 * Dónde queda el centro del botón a lo largo del applet, en píxeles: desde
	 * el borde izquierdo con el panel arriba o abajo, desde el de arriba con el
	 * panel a un costado.
	 */
	origin: number;
}

/**
 * Cuánto hay de cada canto de la superficie al applet: el margen de sombra.
 *
 * La superficie es más grande que el applet para que la sombra no se corte
 * (`SHADOW_BLEED` en `anchored_applet.rs`), y el backend dice cuánto quedó de
 * cada lado: contra el borde del monitor puede ser menos que el resto.
 */
export interface AppletInset {
	left: number;
	right: number;
	top: number;
	bottom: number;
}

/** Sin margen: el applet ocupa la superficie entera. */
export const NO_INSET: AppletInset = { left: 0, right: 0, top: 0, bottom: 0 };

/** El evento con que el backend avisa que un applet se mostró. */
export interface AppletShownEvent extends AppletAnchor {
	applet: string;
	inset?: unknown;
}

const INSET_EDGES = ['left', 'right', 'top', 'bottom'] as const;

const toEdge = (value: unknown): number =>
	typeof value === 'number' && Number.isFinite(value) && value > 0 ? value : 0;

/**
 * El margen que mandó el backend, o ninguno si no se entiende.
 *
 * Un lado que no es un número vale cero: dibujar el applet contra el canto es
 * lo que pasaba antes del margen, y es mejor que un `NaNpx` que el navegador
 * descarta y deja el applet sin posición.
 */
export function toInset(value: unknown): AppletInset {
	if (!value || typeof value !== 'object') return { ...NO_INSET };
	const raw = value as Record<string, unknown>;
	const inset = { ...NO_INSET };
	for (const edge of INSET_EDGES) inset[edge] = toEdge(raw[edge]);
	return inset;
}

/**
 * El margen que viene en la ruta de la primera apertura:
 * `inset=izquierda,derecha,arriba,abajo`.
 */
export function insetFromQuery(query: Record<string, unknown>): AppletInset {
	const raw = Array.isArray(query.inset) ? query.inset[0] : query.inset;
	if (typeof raw !== 'string') return { ...NO_INSET };
	const values = raw.split(',').map((part) => Number.parseFloat(part));
	if (values.length !== INSET_EDGES.length) return { ...NO_INSET };
	const [left, right, top, bottom] = values;
	return toInset({ left, right, top, bottom });
}

/** El margen como posición del applet dentro de la superficie. */
export function insetStyle(inset: AppletInset): Record<string, string> {
	return {
		left: `${inset.left}px`,
		right: `${inset.right}px`,
		top: `${inset.top}px`,
		bottom: `${inset.bottom}px`,
	};
}

/**
 * El `transform-origin` de la entrada.
 *
 * Sobre el eje del panel, el botón; sobre el otro, el borde que toca el panel.
 * Con el panel arriba el applet crece hacia abajo desde su borde de arriba; con
 * el panel a la derecha, hacia la izquierda desde su borde derecho.
 *
 * Sin anclaje —la página se abrió sin que el backend dijera nada— crece desde
 * el centro del borde del panel de arriba, que es donde está el panel por
 * omisión.
 */
export function appletTransformOrigin(anchor: AppletAnchor | null): string {
	if (!anchor) return '50% 0%';

	const along = Number.isFinite(anchor.origin) ? `${Math.max(0, anchor.origin)}px` : '50%';

	switch (anchor.side) {
		case 'top':
			return `${along} 0%`;
		case 'bottom':
			return `${along} 100%`;
		case 'left':
			return `0% ${along}`;
		case 'right':
			return `100% ${along}`;
	}
}

/**
 * El anclaje que viene en la ruta.
 *
 * La primera vez que se abre un applet la página todavía no existe cuando el
 * backend avisa, así que el lado y el origen llegan en la consulta de la ruta
 * (`#/applets/audio?side=top&origin=120`). Un valor que no se entiende se
 * descarta entero: un origen sin lado no dice de qué borde crecer.
 */
export function anchorFromQuery(query: Record<string, unknown>): AppletAnchor | null {
	const side = Array.isArray(query.side) ? query.side[0] : query.side;
	const rawOrigin = Array.isArray(query.origin) ? query.origin[0] : query.origin;
	const origin = typeof rawOrigin === 'string' ? Number.parseFloat(rawOrigin) : Number.NaN;

	if (!isPanelPosition(side) || !Number.isFinite(origin)) return null;
	return { side, origin };
}

/** El rectángulo que se manda al backend: lo mismo que `DOMRect`, sin métodos. */
export interface AnchorRect {
	x: number;
	y: number;
	width: number;
	height: number;
}

/**
 * El rectángulo de un botón del panel, en píxeles lógicos del panel.
 *
 * Acepta el elemento o el componente (lo que da un `ref` sobre un componente de
 * la librería), y devuelve `undefined` si no hay nada que medir: el backend
 * centra entonces el applet en el eje del panel en vez de fallar.
 */
export function anchorOf(target: unknown): AnchorRect | undefined {
	const element =
		target && typeof target === 'object' && '$el' in target
			? (target as { $el: unknown }).$el
			: target;

	if (!element || typeof (element as Element).getBoundingClientRect !== 'function') {
		return undefined;
	}

	const { x, y, width, height } = (element as Element).getBoundingClientRect();
	return { x, y, width, height };
}
