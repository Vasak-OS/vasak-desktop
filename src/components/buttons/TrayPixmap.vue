<script setup lang="ts">
/**
 * El mapa de bits que manda **otra aplicación**: el `IconPixmap` de un
 * elemento de la bandeja o el `icon-data` de una entrada de su menú.
 *
 * Es la única excepción a «los iconos salen del tema» (la guardia de diseño la
 * nombra por este archivo): no es un icono nuestro, es el de un tercero, y el
 * tema no lo puede tener. Llega ya convertido a PNG y con tope de tamaño
 * (`tray/pixmap.rs`), en base64. Si no hay datos no dibuja nada.
 */
import { computed } from 'vue';

const props = withDefaults(
	defineProps<{
		/** PNG en base64. */
		data?: string;
		/** Lado en píxeles lógicos. */
		size?: number;
		alt?: string;
	}>(),
	{ size: 16, alt: '' }
);

const source = computed(() => (props.data ? `data:image/png;base64,${props.data}` : ''));
</script>

<template>
  <img
    v-if="source"
    :src="source"
    :alt="alt"
    :width="size"
    :height="size"
    class="shrink-0 object-contain"
    :style="{ width: `${size}px`, height: `${size}px` }"
  />
</template>
