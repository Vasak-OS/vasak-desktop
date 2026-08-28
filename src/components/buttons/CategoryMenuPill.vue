<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { computed } from 'vue';
import { useIcon } from '@/tools/composables/useReactiveIcon';

const emit = defineEmits(['update:categorySelected']);

const props = defineProps<{
	category: any;
	image: string;
	/** Nombre de la categoría, ya traducido: es lo único que nombra al botón. */
	label: string;
	categorySelected: string;
	large?: boolean;
}>();

const appIcon = useIcon(computed(() => props.image));

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
    <img :src="appIcon" alt="" :class="large ? 'h-14' : 'h-10'" />
  </button>
</template>

