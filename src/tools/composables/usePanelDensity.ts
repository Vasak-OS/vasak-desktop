import { onBeforeUnmount, onMounted, type Ref, ref } from 'vue';
import { type PanelDensity, panelDensity } from '@/tools/panel-density';

/**
 * Cuánto lugar tiene el panel, compartido por todas sus píldoras.
 *
 * Una sola referencia para todo el panel, como `useOpenApplet`: la barra la
 * mide (`watchPanelDensity`) y cada píldora la lee (`usePanelDensity`) para
 * decidir si muestra su texto o queda en el icono.
 */
const density = ref<PanelDensity>('full');

export function usePanelDensity(): Ref<PanelDensity> {
	return density;
}

/**
 * Mide la barra con `ResizeObserver` —en este WebView `resize` no llega— y
 * actualiza la densidad cuando cruza un umbral.
 */
export function watchPanelDensity(bar: Ref<HTMLElement | null>, vertical: Ref<boolean>): void {
	let observer: ResizeObserver | undefined;

	const measure = () => {
		const element = bar.value;
		if (!element) return;
		const box = element.getBoundingClientRect();
		density.value = panelDensity(vertical.value ? box.height : box.width);
	};

	onMounted(() => {
		measure();
		if (typeof ResizeObserver !== 'undefined' && bar.value) {
			observer = new ResizeObserver(measure);
			observer.observe(bar.value);
		}
	});

	onBeforeUnmount(() => observer?.disconnect());
}

/** Para las pruebas: la referencia compartida. */
export const panelDensityForTests = density;
