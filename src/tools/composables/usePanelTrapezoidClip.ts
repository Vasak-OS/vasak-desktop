import { onBeforeUnmount, onMounted, type Ref, watch } from 'vue';
import type { PanelPosition } from '@/tools/panel-position';
import { trapezoidClipPath } from '@/tools/panel-trapezoid';

/**
 * Recorta la superficie del panel con la forma de trapecio redondeado cuando el
 * tipo es `trapezoid`. La forma lleva arcos (scoop cóncavo en la parte ancha,
 * convexo en la angosta), así que no sale de un `polygon()`: se arma un
 * `path()` en píxeles con `panel-trapezoid.ts` y se vuelve a calcular cada vez
 * que cambia el tamaño del panel —con `ResizeObserver`, porque en WebKitGTK el
 * evento `resize` no llega—. No hay realimentación: recortar no cambia el tamaño
 * del elemento, así que medir después de aplicar da lo mismo (a diferencia del
 * `w-fit` que medía `watchPanelDensity`).
 *
 * Cuando no es trapecio se limpia el `clip-path` en línea para no pisar la clase
 * de la superficie (barra/flotante/dock, que redondean por CSS).
 */
export function usePanelTrapezoidClip(
	el: Ref<HTMLElement | null>,
	position: Ref<PanelPosition>,
	enabled: Ref<boolean>
): void {
	let observer: ResizeObserver | undefined;

	const apply = () => {
		const node = el.value;
		if (!node) return;
		if (!enabled.value) {
			node.style.clipPath = '';
			return;
		}
		const rect = node.getBoundingClientRect();
		node.style.clipPath = trapezoidClipPath(position.value, rect.width, rect.height);
	};

	onMounted(() => {
		if (typeof ResizeObserver !== 'undefined') observer = new ResizeObserver(apply);

		// La superficie aparece y desaparece con el tipo (`v-if="hasSurface"`), así
		// que se observa el nodo actual y se reobserva cuando cambia.
		watch(
			el,
			(node, prev) => {
				if (prev && observer) observer.unobserve(prev);
				if (node && observer) observer.observe(node);
				apply();
			},
			{ immediate: true }
		);

		// Cambiar de lado o de tipo en caliente (evento `config-changed`) recalcula.
		watch([position, enabled], apply);
	});

	onBeforeUnmount(() => observer?.disconnect());
}
