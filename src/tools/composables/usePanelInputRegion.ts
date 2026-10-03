import { onBeforeUnmount, onMounted, type Ref } from 'vue';
import { type InputRect, setPanelInputRegion } from '@/services/compositor.service';
import { PILL_SELECTOR, pillRects, sameRects } from '@/tools/panel-input-region';
import { logError } from '@/utils/logger';

/**
 * Mantiene la región de entrada del panel recortada a sus píldoras.
 *
 * Vuelve a medir cuando una píldora cambia de tamaño (un título de canción
 * más largo, el panel que pasa a un costado) y cuando aparece o desaparece
 * una (el teléfono, la bandeja vacía). Con `ResizeObserver` y
 * `MutationObserver`, no con `resize`: en este WebView `resize` no llega.
 * Los pedidos se juntan en un cuadro y sólo salen si los rectángulos
 * cambiaron.
 */
export function usePanelInputRegion(root: Ref<HTMLElement | null>): void {
	let sizes: ResizeObserver | undefined;
	let tree: MutationObserver | undefined;
	let frame = 0;
	let sent: InputRect[] = [];

	const contains = (outer: unknown, inner: unknown) =>
		outer instanceof Node && inner instanceof Node && outer.contains(inner);

	const pills = (): HTMLElement[] =>
		root.value ? [...root.value.querySelectorAll<HTMLElement>(PILL_SELECTOR)] : [];

	const report = () => {
		frame = 0;
		const rects = pillRects(pills(), contains);
		if (sameRects(rects, sent)) return;
		sent = rects;
		setPanelInputRegion(rects).catch((error) =>
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

	onBeforeUnmount(() => {
		sizes?.disconnect();
		tree?.disconnect();
		if (frame) cancelAnimationFrame(frame);
	});
}
