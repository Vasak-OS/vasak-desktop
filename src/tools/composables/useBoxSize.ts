import { onMounted, onUnmounted, type Ref, ref } from 'vue';

/**
 * El tamaño de la caja de un widget, medido.
 *
 * En el WebView no llegan ni `matchMedia` ni el `resize` de la ventana, así que
 * lo que se mira es la caja propia. De acá salen qué secciones entran
 * (vasak-desktop#269); mientras no se midió, vale 0.
 */
export function useBoxSize(box: Ref<HTMLElement | null>) {
	const width = ref(0);
	const height = ref(0);
	let observer: ResizeObserver | null = null;

	const measure = () => {
		width.value = box.value?.clientWidth ?? 0;
		height.value = box.value?.clientHeight ?? 0;
	};

	onMounted(() => {
		if (!box.value) return;
		observer = new ResizeObserver(measure);
		observer.observe(box.value);
		measure();
	});
	onUnmounted(() => observer?.disconnect());

	return { width, height };
}
