<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { computed } from 'vue';
import { useIcon } from '@/tools/composables/useReactiveIcon';

const emit = defineEmits(['update:categorySelected']);

const props = defineProps<{
	category: any;
	image: string;
	description: string;
	categorySelected: string;
}>();

const appIcon = useIcon(computed(() => props.image));

const setCategory = (category: string) => {
	emit('update:categorySelected', category);
};
</script>

<template>
  <button
    class="w-full h-full flex items-center justify-center p-2 rounded-corner hover:scale-110 transition-transform duration-200"
    @click="setCategory(category)"
:class="[
    category === categorySelected
      ? 'bg-primary border border-secondary relative'
      : 'bg-transparent border border-transparent hover:bg-ui-surface/60'
  ]"
  >
    <img :src="appIcon" :title="description" :alt="category" class="h-10" />
  </button>
</template>

