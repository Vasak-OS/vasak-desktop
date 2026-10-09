<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { marqueeShift } from '@/tools/marquee';

/**
 * Un texto a ancho de su celda que se desplaza en horizontal **sólo si no
 * entra** (vasak-desktop#203). Reemplaza al `truncate` seco de los nombres del
 * menú: un nombre largo sin espacios se podía leer entero pasando el puntero por
 * encima, sin romper la grilla.
 *
 * En reposo el texto queda recortado con puntos suspensivos. Si sobra —lo mide
 * un `ResizeObserver` sobre la caja y el contenido, porque en WebKitGTK ni
 * `matchMedia` ni `resize` avisan— al pasar el puntero se desplaza para mostrar
 * el final y vuelve, en bucle suave. Respeta a quien pidió menos movimiento
 * (`prefers-reduced-motion`): ahí no se mueve y se queda con los puntos.
 *
 * La caja nunca excede su hueco (`w-full`, `overflow-hidden`), así que no empuja
 * ni rompe la celda de la grilla por más largo que sea el nombre.
 */
const props = defineProps<{ text: string }>();

const root = ref<HTMLElement | null>(null);
const inner = ref<HTMLElement | null>(null);
/** Los píxeles que sobran, para el `clip-path`/desplazamiento; cero si entra. */
const shift = ref(0);

let observer: ResizeObserver | undefined;

function measure(): void {
	const box = root.value;
	const content = inner.value;
	if (!box || !content) return;
	shift.value = marqueeShift(content.scrollWidth, box.clientWidth);
}

onMounted(() => {
	if (typeof ResizeObserver !== 'undefined') {
		observer = new ResizeObserver(() => measure());
		if (root.value) observer.observe(root.value);
		if (inner.value) observer.observe(inner.value);
	}
	measure();
});

// El texto puede cambiar (otra categoría, otra búsqueda): se vuelve a medir.
watch(
	() => props.text,
	() => queueMicrotask(measure)
);

onBeforeUnmount(() => observer?.disconnect());
</script>

<template>
  <span
    ref="root"
    class="marquee block w-full overflow-hidden"
    :class="{ 'marquee--overflow': shift > 0 }"
    :style="{ '--marquee-shift': `-${shift}px` }"
    :title="text"
    data-marquee
  >
    <span ref="inner" class="marquee__inner" data-marquee-inner>{{ text }}</span>
  </span>
</template>

<style scoped>
/*
 * En reposo: una sola línea recortada con puntos suspensivos, sin exceder la
 * caja. El contenido mide su ancho natural para poder saber si sobra.
 */
.marquee {
	white-space: nowrap;
}

.marquee__inner {
	display: inline-block;
	max-width: 100%;
	overflow: hidden;
	text-overflow: ellipsis;
	vertical-align: bottom;
	white-space: nowrap;
}

/*
 * Cuando sobra y se pasa el puntero: el contenido se suelta (sin recorte) y se
 * desplaza para mostrar el final, de ida y de vuelta, en bucle. Vue reescribe el
 * nombre del `@keyframes` dentro de este bloque, así que queda contenido acá.
 */
.marquee--overflow:hover .marquee__inner {
	max-width: none;
	overflow: visible;
	text-overflow: clip;
	animation: marquee-scroll 5s ease-in-out infinite;
}

@keyframes marquee-scroll {
	0%,
	12% {
		transform: translateX(0);
	}
	44%,
	56% {
		transform: translateX(var(--marquee-shift, 0));
	}
	88%,
	100% {
		transform: translateX(0);
	}
}

/* Menos movimiento para quien lo pidió (WCAG 2.3.3): no se desplaza. */
@media (prefers-reduced-motion: reduce) {
	.marquee--overflow:hover .marquee__inner {
		animation: none;
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
	}
}
</style>
