<script setup lang="ts">
import { computed } from 'vue';
import DesktopClockWidget from '@/components/widgets/DesktopClockWidget.vue';
import FilesWidget from '@/components/widgets/FilesWidget.vue';
import MusicWidget from '@/components/widgets/MusicWidget.vue';
import WeatherWidget from '@/components/widgets/WeatherWidget.vue';
import WidgetFrame from '@/components/widgets/WidgetFrame.vue';
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

const WIDGET_COMPONENTS = {
	clock: DesktopClockWidget,
	music: MusicWidget,
	weather: WeatherWidget,
	files: FilesWidget,
} as const;

const widgetComponent = computed(() => WIDGET_COMPONENTS[props.type]);

/** La forma por omisión del widget, si quien lo pone no eligió una. */
const widgetVariant = computed(() => props.variant ?? WIDGETS[props.type].variants?.[0]?.id);
</script>

<template>
	<!-- La tarjeta de lo que se apoya dentro de otra ventana, y el contenedor
	     contra el que se miden las unidades de adentro: es `WidgetFrame` con
	     la superficie `surface`, la misma caja del escritorio. -->
	<WidgetFrame surface="surface">
		<component :is="widgetComponent" :variant="widgetVariant" />
	</WidgetFrame>
</template>
