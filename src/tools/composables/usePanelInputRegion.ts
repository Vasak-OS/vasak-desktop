import { onBeforeUnmount, onMounted, type Ref, watch } from 'vue';
import { type InputRect, setPanelInputRegion } from '@/services/compositor.service';
import { type PanelRegionMode, revealStripRect } from '@/tools/panel-autohide';
import { PILL_SELECTOR, pillRects, sameRects } from '@/tools/panel-input-region';
import type { PanelPosition } from '@/tools/panel-position';
import { logError } from '@/utils/logger';

/**
 * Mantiene la región de entrada del panel recortada a lo que se ve, según el
 * estado (`panel-autohide.ts`):
 *
 * - `pills`: la región son las píldoras. Entre una y otra se ve el escritorio y
 *   un clic ahí tiene que caer en él (lo de siempre, sin superficie).
 * - `bar`: la barra entera. Cuando el tipo dibuja una superficie continua
 *   —flotante, barra, dock— no hay huecos, y con auto-ocultar la barra revelada
 *   se trata como un bloque para que moverse entre píldoras no la esconda.
 * - `hidden`: sólo la línea fina del borde. La barra está escondida y todo lo
 *   demás de la franja atraviesa hasta la ventana de abajo; tocar la línea la
 *   revela.
 *
 * Vuelve a medir cuando una píldora cambia de tamaño (un título de canción
 * más largo, el panel que pasa a un costado), cuando aparece o desaparece una
 * (el teléfono, la bandeja vacía), y cuando cambia el estado o el lado. Con
 * `ResizeObserver` y `MutationObserver`, no con `resize`: en este WebView
 * `resize` no llega. Los pedidos se juntan en un cuadro y sólo salen si los
 * rectángulos cambiaron.
 */
export function usePanelInputRegion(
	root: Ref<HTMLElement | null>,
	mode?: Ref<PanelRegionMode>,
	position?: Ref<PanelPosition>
): void {
	let sizes: ResizeObserver | undefined;
	let tree: MutationObserver | undefined;
	let frame = 0;
	let sent: InputRect[] = [];

	const contains = (outer: unknown, inner: unknown) =>
		outer instanceof Node && inner instanceof Node && outer.contains(inner);

	const pills = (): HTMLElement[] =>
		root.value ? [...root.value.querySelectorAll<HTMLElement>(PILL_SELECTOR)] : [];

	/** Los rectángulos que recibe el puntero en el estado actual. */
	const rects = (): InputRect[] => {
		const current = mode?.value ?? 'pills';
		if (typeof window !== 'undefined') {
			if (current === 'hidden' && position) {
				return [revealStripRect(position.value, window.innerWidth, window.innerHeight)];
			}
			// La franja entera: la ventana del panel es la superficie.
			if (current === 'surface') {
				return [{ x: 0, y: 0, width: window.innerWidth, height: window.innerHeight }];
			}
		}
		if (current === 'bar') {
			return root.value ? pillRects([root.value], contains) : [];
		}
		return pillRects(pills(), contains);
	};

	const report = () => {
		frame = 0;
		const next = rects();
		if (sameRects(next, sent)) return;
		sent = next;
		setPanelInputRegion(next).catch((error) =>
			logError('[panel] no se pudo recortar la región de entrada:', error)
		);
	};

	const schedule = () => {
		if (frame) return;
		frame = requestAnimationFrame(report);
	};

	const observePills = () => {
		if (!sizes) return;
		sizes.disconnect();
		if (root.value) sizes.observe(root.value);
		for (const pill of pills()) sizes.observe(pill);
		schedule();
	};

	onMounted(() => {
		if (typeof ResizeObserver !== 'undefined') sizes = new ResizeObserver(schedule);
		if (typeof MutationObserver !== 'undefined' && root.value) {
			tree = new MutationObserver(observePills);
			tree.observe(root.value, { childList: true, subtree: true });
		}
		observePills();
		schedule();
	});

	// Cambiar de estado (esconder, revelar, pasar a una superficie continua) o de
	// lado cambia qué se mide, aunque ninguna píldora se haya movido: hay que
	// volver a pedir.
	if (mode) watch(mode, schedule);
	if (position) watch(position, schedule);

	onBeforeUnmount(() => {
		sizes?.disconnect();
		tree?.disconnect();
		if (frame) cancelAnimationFrame(frame);
	});
}
