<template>
  <component
    :is="interactive ? 'button' : 'div'"
    v-bind="interactive ? { type: 'button', 'aria-label': nombreAccesible } : {}"
    class="theme-transition p-1 rounded-corner relative group transition-all duration-300"
    :class="[customClass, interactive ? 'cursor-pointer hover:bg-primary' : '']"
    :title="tooltip"
    @click="handleClick"
    @mouseenter="showTooltip = true"
    @mouseleave="showTooltip = false"
  >
    <ThemeIcon
      :name="name"
      :type="type"
      :size="22"
      :alt="alt"
      class="m-auto transition-all duration-300"
      :class="iconClass"
    />
    
    <!-- Badge/Counter -->
    <div
      v-if="badge !== null && badge > 0"
      class="absolute bottom-1 right-1 bg-primary text-tx-on-primary text-xs rounded-full w-4 h-4 flex items-center justify-center font-bold animate-bounce"
    >
      {{ badge }}
    </div>

    <!-- Tooltip personalizado -->
    <div 
      v-if="showCustomTooltip && customTooltipText"
      class="absolute top-1 left-1/2 transform -translate-x-1/2 text-xs font-semibold p-1 rounded-corner transition-all duration-300 pointer-events-none bg-ui-bg/80"
      :class="[
        tooltipClass,
        {
          'opacity-0 -translate-y-2': !showTooltip,
          'opacity-100 translate-y-0': showTooltip
        }
      ]"
    >
      {{ customTooltipText }}
    </div>

    <!-- Slot para contenido adicional personalizado -->
    <slot></slot>
  </component>
</template>

<script setup lang="ts">
/**
 * Un icono de la bandeja del panel.
 *
 * El icono se pide por **nombre** y no por ruta: antes recibía la ruta ya
 * resuelta, así que cada botón tenía que resolverla por su cuenta y volver a
 * pedirla al cambiar el tema. `ThemeIcon` hace eso una sola vez para toda la
 * ventana, y además entra en el planificador de recarga: el que está en pantalla
 * se recarga antes que el que no.
 */
/** biome-ignore-all lint/correctness/noUnusedVariables: <User in template> */
import { ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, ref } from 'vue';

interface Props {
	/** El nombre del icono en el tema del escritorio. */
	name: string;
	/** Cuál de las dos variantes del tema. La bandeja usa el glifo monocromo. */
	type?: 'icon' | 'symbol';
	alt?: string;
	tooltip?: string;
	badge?: number | null;
	iconClass?: string | Record<string, boolean>;
	customClass?: string | Record<string, boolean>;
	tooltipClass?: string | Record<string, boolean>;
	showCustomTooltip?: boolean;
	customTooltipText?: string;
	/**
	 * Si hacer clic hace algo.
	 *
	 * Los que sólo informan —la batería, Bloq Mayús, el micrófono silenciado—
	 * se pintaban al pasar el mouse como si fueran botones: el resaltado promete
	 * un clic que no existe. Con esto quedan quietos.
	 */
	interactive?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
	type: 'symbol',
	alt: '',
	tooltip: '',
	badge: null,
	iconClass: () => ({}),
	customClass: '',
	tooltipClass: '',
	showCustomTooltip: false,
	customTooltipText: '',
	interactive: true,
});

const emit = defineEmits<{
	click: [];
}>();

/**
 * Cómo se llama el botón para quien no ve el icono.
 *
 * El dibujo es todo el contenido: sin esto el lector de pantalla anuncia un
 * botón vacío. Se usa el `alt` del icono, y si no hay, el texto del tooltip.
 */
const nombreAccesible = computed(() => props.alt || props.tooltip || undefined);

const showTooltip = ref(false);

const handleClick = () => {
	emit('click');
};
</script>
