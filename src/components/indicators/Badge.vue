<script setup lang="ts">
/**
 * Copia fina y **provisoria** de la `Badge` de vue-libvasak 2.1.0.
 *
 * La insignia de la librería llega en la 2.1.0 (vue-libvasak#76, abierto al
 * escribir esto). Hasta que se publique, el contador de la bandeja la necesita
 * y no se puede dibujar a mano con otro estilo: esto es **la misma**
 * insignia —mismas clases, mismos nombres de propiedad— reducida a lo que usa
 * el escritorio (`tone`, `variant`, `size`, `label`). Cuando el escritorio
 * suba a la 2.1.0, se borra este archivo y se importa `Badge` desde
 * `@vasakgroup/vue-libvasak`, sin tocar a quien la usa.
 */
import { computed } from 'vue';

export type BadgeTone = 'neutral' | 'accent' | 'warning' | 'error';

const props = withDefaults(
	defineProps<{
		tone?: BadgeTone;
		variant?: 'soft' | 'solid';
		size?: 'sm' | 'md';
		label?: string | number;
	}>(),
	{ tone: 'neutral', variant: 'soft', size: 'sm' }
);

/** Las mismas tablas de la librería, para estos tonos y variantes. */
const FILL: Record<'soft' | 'solid', Record<BadgeTone, string>> = {
	soft: {
		neutral: 'bg-ui-selected',
		accent: 'bg-ui-selected-accent',
		warning: 'bg-status-warning/15',
		error: 'bg-status-error/15',
	},
	solid: {
		neutral: 'bg-ui-pressed',
		accent: 'bg-primary',
		warning: 'bg-status-warning/15',
		error: 'bg-status-error/15',
	},
};

const fillClass = computed(() => FILL[props.variant][props.tone]);
const textClass = computed(() =>
	props.variant === 'solid' && props.tone === 'accent' ? 'text-tx-on-primary' : 'text-tx-main'
);
const sizeClass = computed(() =>
	props.size === 'md' ? 'min-h-6 px-2 text-label-s' : 'min-h-5 px-2 text-label-xs'
);
</script>

<template>
  <span
    class="inline-flex max-w-full min-w-0 shrink-0 items-center gap-1 rounded-corner-full border border-transparent font-semibold break-words"
    :class="[fillClass, textClass, sizeClass]"
  >
    <span class="min-w-0"><slot>{{ label }}</slot></span>
  </span>
</template>
