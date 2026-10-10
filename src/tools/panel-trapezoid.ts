import type { PanelPosition } from '@/tools/panel-position';

/**
 * El recorte del panel en forma de trapecio invertido (vasak-desktop#219): la
 * **parte ancha** —la que se apoya contra el borde de la pantalla— remata en un
 * **saliente cóncavo** (un «radio invertido»: el canto se abre hacia el borde,
 * como la base de una pestaña de navegador), y la **parte angosta** —el borde de
 * adentro— lleva **esquinas convexas** redondeadas. Los lados caen en diagonal,
 * del ancho de arriba al angosto de abajo.
 *
 * Se devuelve como `clip-path: path(...)` y no como una clase de CSS porque la
 * forma tiene curvas (béziers): un `polygon()` no las hace. El `path()` lleva
 * coordenadas en píxeles, así que el que llama lo recalcula cuando cambia el
 * tamaño del panel (`usePanelTrapezoidClip`). No realimenta ningún
 * `ResizeObserver`: recortar no cambia el tamaño del elemento.
 */

/** Cuánto entra en diagonal cada costado, del ancho de arriba al angosto de abajo. */
export const CHAMFER = 22;
/** Ancho del saliente cóncavo en la parte ancha (contra el borde de pantalla). */
export const FLARE = 12;
/** Cuánto baja el saliente cóncavo antes de empezar la diagonal recta. */
export const FLARE_DROP = 14;
/** Radio convexo de las esquinas de la parte angosta (el borde de adentro). */
export const CONVEX = 7;

/** Un punto en el plano del recorte. */
type Point = [number, number];
/** Un tramo del contorno: `M`/`L` llevan un punto; `Q` uno de control + destino;
 * `C` dos de control + destino. */
type Segment =
	| { cmd: 'M' | 'L'; pts: [Point] }
	| { cmd: 'Q'; pts: [Point, Point] }
	| { cmd: 'C'; pts: [Point, Point, Point] };

const sub = (a: Point, b: Point): Point => [a[0] - b[0], a[1] - b[1]];
const unit = (a: Point): Point => {
	const l = Math.hypot(a[0], a[1]) || 1;
	return [a[0] / l, a[1] / l];
};
const along = (from: Point, dir: Point, d: number): Point => [
	from[0] + dir[0] * d,
	from[1] + dir[1] * d,
];

/**
 * Arma el contorno en orientación «canónica»: barra horizontal, lado ancho
 * arriba (`y` 0), lado angosto abajo (`y` = D), de largo `L`. El resto de las
 * posiciones salen de transformar estos puntos —incluidos los de control—, así
 * que las curvas se reorientan solas sin tocar sweeps.
 *
 * Recorrido en sentido horario (Y hacia abajo), empezando en la esquina de
 * arriba a la izquierda (contra la pantalla): saliente cóncavo arriba-izquierda →
 * diagonal izquierda → convexo abajo-izquierda → borde angosto → convexo
 * abajo-derecha → diagonal derecha → saliente cóncavo arriba-derecha.
 */
function canonical(
	L: number,
	D: number,
	chamfer: number,
	flare: number,
	drop: number,
	convex: number
): Segment[] {
	const tl: Point = [0, 0];
	const tr: Point = [L, 0];
	const dLtop: Point = [flare, drop]; // arranque de la diagonal izquierda
	const dRtop: Point = [L - flare, drop];
	const bl: Point = [chamfer, D]; // vértice nominal abajo-izquierda
	const br: Point = [L - chamfer, D];

	// Esquinas convexas de abajo: se corta `convex` sobre la diagonal y sobre la
	// base inferior, y se redondea con una cuadrática que pasa por el vértice.
	const downL = unit(sub(bl, dLtop));
	const pLdiag = along(bl, downL, -convex);
	const pLbase: Point = [chamfer + convex, D];
	const downR = unit(sub(br, dRtop));
	const pRdiag = along(br, downR, -convex);
	const pRbase: Point = [L - chamfer - convex, D];

	// Saliente cóncavo de arriba: cúbica tangente al borde (horizontal) en la
	// esquina de la pantalla y a la diagonal en su arranque.
	const handle = flare * 0.55;
	const upL = unit(sub(dLtop, bl));
	const cLexit: Point = [handle, 0];
	const cLenter = along(dLtop, upL, -handle);
	const upR = unit(sub(dRtop, br));
	const cRenter = along(dRtop, upR, -handle);
	const cRexit: Point = [L - handle, 0];

	return [
		{ cmd: 'M', pts: [tl] },
		{ cmd: 'C', pts: [cLexit, cLenter, dLtop] },
		{ cmd: 'L', pts: [pLdiag] },
		{ cmd: 'Q', pts: [bl, pLbase] },
		{ cmd: 'L', pts: [pRbase] },
		{ cmd: 'Q', pts: [br, pRdiag] },
		{ cmd: 'L', pts: [dRtop] },
		{ cmd: 'C', pts: [cRenter, cRexit, tr] },
	];
}

/**
 * Lleva la orientación canónica (largo en X, ancho arriba) a cada posición. Como
 * se transforman todos los puntos —vértices y controles—, las curvas quedan bien
 * orientadas sin invertir nada.
 *
 * - `top`: identidad.
 * - `bottom`: espejo vertical (el lado ancho pasa abajo).
 * - `left`/`right`: gira 90° (el largo pasa al eje Y, el ancho contra su borde).
 */
function place(
	position: PanelPosition,
	w: number,
	h: number
): { map: (p: Point) => Point; long: number; depth: number } {
	switch (position) {
		case 'top':
			return { map: ([x, y]) => [x, y], long: w, depth: h };
		case 'bottom':
			return { map: ([x, y]) => [x, h - y], long: w, depth: h };
		case 'left':
			return { map: ([x, y]) => [y, x], long: h, depth: w };
		case 'right':
			return { map: ([x, y]) => [w - y, x], long: h, depth: w };
	}
}

const fmt = (n: number) => Math.round(n * 100) / 100;

/**
 * El valor para `clip-path` del trapecio invertido en el tamaño y la posición
 * dados, o `'none'` si el elemento todavía no tiene tamaño.
 */
export function trapezoidClipPath(
	position: PanelPosition,
	w: number,
	h: number,
	opts: { chamfer?: number; flare?: number; drop?: number; convex?: number } = {}
): string {
	if (w <= 0 || h <= 0) return 'none';
	const { map, long, depth } = place(position, w, h);

	// Acota todo al espacio real: un panel angosto o cortito no puede llevar el
	// bisel ni los radios enteros sin que las curvas se crucen o den NaN.
	const chamfer = Math.max(0, Math.min(opts.chamfer ?? CHAMFER, (long - 2) / 2));
	const flare = Math.max(0, Math.min(opts.flare ?? FLARE, chamfer));
	const drop = Math.max(0, Math.min(opts.drop ?? FLARE_DROP, depth * 0.6));
	const diag = Math.hypot(chamfer - flare, depth - drop);
	const convex = Math.max(
		0,
		Math.min(opts.convex ?? CONVEX, diag, Math.max(0, (long - 2 * chamfer) / 2))
	);

	const parts = canonical(long, depth, chamfer, flare, drop, convex).map((seg) => {
		const coords = seg.pts.map((p) => {
			const [x, y] = map(p);
			return `${fmt(x)},${fmt(y)}`;
		});
		return `${seg.cmd} ${coords.join(' ')}`;
	});

	return `path('${parts.join(' ')} Z')`;
}
