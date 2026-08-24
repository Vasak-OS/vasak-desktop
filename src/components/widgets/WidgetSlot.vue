<script setup lang="ts">
import { computed } from 'vue';
import DesktopClockWidget from '@/components/widgets/DesktopClockWidget.vue';
import FilesWidget from '@/components/widgets/FilesWidget.vue';
import MusicWidget from '@/components/widgets/MusicWidget.vue';
import WeatherWidget from '@/components/widgets/WeatherWidget.vue';
import { WIDGETS, type WidgetType } from '@/tools/widgets/catalog';

/**
 * Un widget del escritorio, fuera de la cuadrícula.
 *
 * Los widgets miden todo en unidades de contenedor —`cqmin`, el lado más chico—
 * y quien declara ese contenedor es el marco de la cuadrícula. Puesto en
 * cualquier otro lado, esas medidas se resuelven contra la ventana entera: el
 * clima en el menú se dibujaba con el tamaño equivocado, que es exactamente lo
 * que pasó.
 *
 * Esto es ese marco, sin la parte de arrastrar y redimensionar. Cualquier widget
 * puesto acá se adapta al hueco que le toque, y cambiar cuál se muestra es
 * cambiar una palabra.
 */
const props = withDefaults(
	defineProps<{
		type?: WidgetType;
		/** Algunos se muestran de más de una forma; el clima, por ejemplo. */
		variant?: string;
	}>(),
	{ type: 'weather', variant: undefined }
);

const componentes = {
	clock: DesktopClockWidget,
	music: MusicWidget,
	weather: WeatherWidget,
	files: FilesWidget,
} as const;

const componente = computed(() => componentes[props.type]);

/** La forma por omisión del widget, si quien lo pone no eligió una. */
const variante = computed(() => props.variant ?? WIDGETS[props.type].variants?.[0]?.id);
</script>

<template>
	<!-- El mismo marco que en el escritorio: fondo, blur, borde y esquinas, y el
	     contenedor contra el que se miden las unidades de adentro. -->
	<div
		style="container-type: size"
		class="h-full w-full overflow-hidden rounded-corner border border-ui-border bg-ui-bg/80 backdrop-blur-md"
	>
		<component :is="componente" :variant="variante" />
	</div>
</template>
