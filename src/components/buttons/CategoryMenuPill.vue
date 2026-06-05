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
    class="p-2 rounded-corner hover:scale-120"
    @click="setCategory(category)"
:class="[
    category === categorySelected
      ? 'bg-primary border border-secondary relative'
      : 'bg-transparent'
  ]"
  >
    <img :src="appIcon" :title="description" :alt="category" class="h-12" />
  </button>
</template>

