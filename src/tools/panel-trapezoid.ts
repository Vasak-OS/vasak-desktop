import type { PanelPosition } from '@/tools/panel-position';

/**
 * El recorte del panel en forma de trapecio (vasak-desktop#219), ahora con los
 * cantos redondeados como en el concepto RYOKU: la **parte ancha** —la que se
 * apoya contra el borde de la pantalla— lleva un radio **cóncavo/invertido**
 * (un «scoop»: el canto se curva hacia afuera uniéndose a la diagonal), y la
 * **parte angosta** —el borde de adentro— un radio **convexo** chico.
 *
 * Se devuelve como `clip-path: path(...)` y no como una clase de CSS porque la
 * forma tiene arcos: un `polygon()` no redondea. El `path()` lleva coordenadas en
 * píxeles, así que el que llama lo recalcula cuando cambia el tamaño del panel
 * (`usePanelTrapezoidClip`). No realimenta ningún `ResizeObserver`: recortar no
 * cambia el tamaño del elemento.
 */

/** Cuánto se come cada costado en diagonal; igual al relleno lateral del contenido. */
export const CHAMFER = 16;
/** Radio del scoop en la parte ancha (contra el borde de pantalla). */
export const WIDE_RADIUS = 14;
/** Radio convexo en la parte angosta (el borde de adentro). */
export const NARROW_RADIUS = 5;

/** Un punto en el plano del recorte. */
type Point = [number, number];

/**
 * Arma el path en orientación «canónica»: barra horizontal, lado ancho arriba
 * (`depth` 0), lado angosto abajo (`depth` = D), de largo `L`. El resto de las
 * posiciones salen de transformar estos puntos.
 *
 * Recorrido en sentido horario (coordenadas con la Y hacia abajo), empezando
 * sobre la diagonal izquierda: scoop arriba-izquierda → borde ancho → scoop
 * arriba-derecha → diagonal derecha → convexo abajo-derecha → borde angosto →
 * convexo abajo-izquierda. Todos los arcos van con `sweep=1` en esta orientación.
 */
function canonical(
	L: number,
	D: number,
	chamfer: number,
	rWide: number,
	rNarrow: number
): Array<{ cmd: 'M' | 'L'; p: Point } | { cmd: 'A'; r: number; sweep: 0 | 1; p: Point }> {
	const diag = Math.hypot(chamfer, D);
	const ux = chamfer / diag; // componente «hacia adentro» en x por unidad de diagonal
	const uy = D / diag; // componente «hacia abajo» en y por unidad de diagonal

	const aDiag: Point = [rWide * ux, rWide * uy];
	const aTop: Point = [rWide, 0];
	const bTop: Point = [L - rWide, 0];
	const bDiag: Point = [L - rWide * ux, rWide * uy];
	const cDiag: Point = [L - chamfer + rNarrow * ux, D - rNarrow * uy];
	const cBot: Point = [L - chamfer - rNarrow, D];
	const dBot: Point = [chamfer + rNarrow, D];
	const dDiag: Point = [chamfer - rNarrow * ux, D - rNarrow * uy];

	return [
		{ cmd: 'M', p: aDiag },
		{ cmd: 'A', r: rWide, sweep: 1, p: aTop },
		{ cmd: 'L', p: bTop },
		{ cmd: 'A', r: rWide, sweep: 1, p: bDiag },
		{ cmd: 'L', p: cDiag },
		{ cmd: 'A', r: rNarrow, sweep: 1, p: cBot },
		{ cmd: 'L', p: dBot },
		{ cmd: 'A', r: rNarrow, sweep: 1, p: dDiag },
	];
}

/**
 * La transformación de la orientación canónica (largo en X, ancho arriba) a cada
 * posición, con el signo del determinante: si es negativa, invierte el `sweep` de
 * los arcos. `w`/`h` son el tamaño real del elemento recortado.
 *
 * - `top`: identidad.
 * - `bottom`: espejo vertical (el lado ancho pasa abajo).
 * - `left`/`right`: se gira 90° (el largo pasa al eje Y, el ancho contra su borde).
 */
function place(
	position: PanelPosition,
	w: number,
	h: number
): { map: (p: Point) => Point; flip: boolean; long: number; depth: number } {
	switch (position) {
		case 'top':
			return { map: ([x, y]) => [x, y], flip: false, long: w, depth: h };
		case 'bottom':
			return { map: ([x, y]) => [x, h - y], flip: true, long: w, depth: h };
		case 'left':
			return { map: ([x, y]) => [y, x], flip: true, long: h, depth: w };
		case 'right':
			return { map: ([x, y]) => [w - y, x], flip: false, long: h, depth: w };
	}
}

const fmt = (n: number) => Math.round(n * 100) / 100;

/**
 * El valor para `clip-path` del trapecio redondeado en el tamaño y la posición
 * dados, o `'none'` si el elemento todavía no tiene tamaño.
 */
export function trapezoidClipPath(
	position: PanelPosition,
	w: number,
	h: number,
	opts: { chamfer?: number; wideRadius?: number; narrowRadius?: number } = {}
): string {
	if (w <= 0 || h <= 0) return 'none';
	const { map, flip, long, depth } = place(position, w, h);

	// Acota los radios y el bisel al espacio real: un panel angosto o cortito no
	// puede llevar el radio entero sin que los arcos se crucen.
	const chamfer = Math.max(0, Math.min(opts.chamfer ?? CHAMFER, (long - 2) / 2, depth));
	const rWide = Math.max(
		0,
		Math.min(opts.wideRadius ?? WIDE_RADIUS, depth, (long - 2 * chamfer) / 2)
	);
	const rNarrow = Math.max(0, Math.min(opts.narrowRadius ?? NARROW_RADIUS, depth - rWide));

	const parts = canonical(long, depth, chamfer, rWide, rNarrow).map((seg) => {
		const [x, y] = map(seg.p);
		if (seg.cmd === 'A') {
			const sweep = flip ? ((1 - seg.sweep) as 0 | 1) : seg.sweep;
			return `A ${fmt(seg.r)} ${fmt(seg.r)} 0 0 ${sweep} ${fmt(x)} ${fmt(y)}`;
		}
		return `${seg.cmd} ${fmt(x)} ${fmt(y)}`;
	});

	return `path('${parts.join(' ')} Z')`;
}
