/**
 * Qué parte del panel recibe el puntero (vasak-desktop#151).
 *
 * El panel en píldoras deja ver el escritorio entre una píldora y otra, pero
 * la superficie de capa sigue siendo la franja entera: es la que reserva el
 * lugar para que las ventanas no queden debajo. Sin recortar su región de
 * entrada, un clic en el hueco entre dos píldoras caería en una franja
 * invisible del panel en vez de en el escritorio que se ve ahí.
 *
 * Esto mide las píldoras y arma los rectángulos que se le pasan al backend
 * (`set_panel_input_region`). Aparte del componente para poder probarlo.
 */
import type { InputRect } from '@/services/compositor.service';

/** Lo que marca a una píldora: la raíz de `PanelPill` de vue-libvasak. */
export const PILL_SELECTOR = '[data-panel-pill]';

/** Lo que se mide de cada elemento: su caja en la página. */
export interface Measurable {
	getBoundingClientRect(): { x: number; y: number; width: number; height: number };
}

/**
 * Los rectángulos de las píldoras, sin las que no se ven (ancho o alto cero:
 * escondidas, o sin nada adentro) ni las anidadas dentro de otra, que ya
 * cubre la de afuera.
 */
export function pillRects(
	pills: readonly Measurable[],
	contains?: (outer: Measurable, inner: Measurable) => boolean
): InputRect[] {
	const visible = pills.filter((pill) => {
		const box = pill.getBoundingClientRect();
		return box.width > 0 && box.height > 0;
	});
	const outermost = contains
		? visible.filter((pill) => !visible.some((other) => other !== pill && contains(other, pill)))
		: visible;
	return outermost.map((pill) => {
		const { x, y, width, height } = pill.getBoundingClientRect();
		return { x, y, width, height };
	});
}

/** Si dos listas de rectángulos son la misma: así no se repite el pedido. */
export function sameRects(left: readonly InputRect[], right: readonly InputRect[]): boolean {
	return (
		left.length === right.length &&
		left.every((rect, index) => {
			const other = right[index];
			return (
				other !== undefined &&
				Math.round(rect.x) === Math.round(other.x) &&
				Math.round(rect.y) === Math.round(other.y) &&
				Math.round(rect.width) === Math.round(other.width) &&
				Math.round(rect.height) === Math.round(other.height)
			);
		})
	);
}
