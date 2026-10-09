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
 * Mide el espacio disponible con `ResizeObserver` —en este WebView `resize` no
 * llega— y actualiza la densidad cuando cruza un umbral.
 *
 * **Se mide el viewport (`documentElement`), no la propia barra.** La superficie
 * de capa ancla el panel de borde a borde, así que el viewport es el espacio
 * real que tiene; y los umbrales (`FULL_FROM`/`COMPACT_FROM`, 1400/960) son de
 * escala de pantalla, no de contenido. En distribuido la barra ocupa el ancho de
 * pantalla y daba lo mismo, pero en compacto la `<nav>` es `w-fit`: medirla
 * realimentaba un bucle —medir → plegar textos → cambia el ancho del contenido →
 * el observer vuelve a disparar— que saturaba el hilo del webview
 * («ResizeObserver loop…») e impedía el primer `.configure` de la capa
 * («Timed out waiting for initial .configure»). El viewport no depende del
 * contenido, así que no hay realimentación.
 */
export function watchPanelDensity(vertical: Ref<boolean>): void {
	let observer: ResizeObserver | undefined;

	const measure = () => {
		const root = document.documentElement;
		if (!root) return;
		const box = root.getBoundingClientRect();
		density.value = panelDensity(vertical.value ? box.height : box.width);
	};

	onMounted(() => {
		measure();
		if (typeof ResizeObserver !== 'undefined' && document.documentElement) {
			observer = new ResizeObserver(measure);
			observer.observe(document.documentElement);
		}
	});

	onBeforeUnmount(() => observer?.disconnect());
}

/** Para las pruebas: la referencia compartida. */
export const panelDensityForTests = density;
