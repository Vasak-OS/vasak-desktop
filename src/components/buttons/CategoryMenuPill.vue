<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { ThemeIcon } from '@vasakgroup/vue-libvasak';

const emit = defineEmits(['update:categorySelected']);

const props = defineProps<{
	category: any;
	image: string;
	/** Nombre de la categoría, ya traducido: es lo único que nombra al botón. */
	label: string;
	categorySelected: string;
	large?: boolean;
}>();

const setCategory = (category: string) => {
	emit('update:categorySelected', category);
};
</script>

<template>
  <button
    class="theme-transition w-full h-full flex items-center justify-center p-2 rounded-corner hover:scale-110 transition-transform duration-200"
    @click="setCategory(category)"
    :title="label"
    :aria-label="label"
    :aria-pressed="category === categorySelected"
:class="[
    category === categorySelected
      ? 'bg-primary border border-secondary relative'
      : 'bg-transparent border border-transparent hover:bg-ui-surface/60'
  ]"
  >
    <ThemeIcon :name="image" :size="large ? 56 : 40" />
  </button>
</template>

