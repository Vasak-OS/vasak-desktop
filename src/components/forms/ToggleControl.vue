<template>
  <button
    @click="handleClick"
    class="theme-transition p-2 rounded-corner bg-ui-bg/80 hover:opacity-50 transition-all duration-300 h-17.5 w-17.5 group relative overflow-hidden hover:scale-105 hover:shadow-lg active:scale-95"
    :class="{
      'animate-pulse': isLoading,
      'ring-2 ring-primary': isActive,
      'opacity-60': !isActive,
      ...customClass
    }"
    :disabled="isLoading"
    :title="label"
    :aria-label="label"
    :aria-pressed="pressed"
    :aria-busy="isLoading || undefined"
  >
    <img
      :src="icon"
      alt=""
      class="m-auto w-12.5 h-12.5 transition-all duration-300 group-hover:scale-110 relative z-10"
      :class="{
        'animate-spin': isLoading,
        'filter brightness-75': !isActive,
        'drop-shadow-lg': isActive,
        ...iconClass
      }"
    />
  </button>
</template>

<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
interface Props {
	icon: string;
	/**
	 * Nombre del control, ya traducido. Es obligatorio porque el botón no tiene
	 * más contenido que un icono: sin esto no tiene nombre accesible y un lector
	 * de pantalla sólo puede anunciar «botón».
	 */
	label: string;
	/**
	 * Estado de dos posiciones, cuando el botón realmente alterna algo. Se deja
	 * sin definir en los que abren un panel: `aria-pressed` ahí miente, y
	 * `isActive` es sólo el resaltado visual.
	 */
	pressed?: boolean;
	isActive?: boolean;
	isLoading?: boolean;
	iconClass?: Record<string, boolean>;
	customClass?: Record<string, boolean>;
}

withDefaults(defineProps<Props>(), {
	isActive: false,
	isLoading: false,
	iconClass: () => ({}),
	customClass: () => ({}),
});

const emit = defineEmits<{
	click: [];
}>();

const handleClick = () => {
	emit('click');
};
</script>
