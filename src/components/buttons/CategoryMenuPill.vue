<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { onMounted, type Ref, ref } from 'vue';

const emit = defineEmits(['update:categorySelected']);

const props = defineProps<{
	category: any;
	image: string;
	description: string;
	categorySelected: string;
}>();

const appIcon: Ref<string> = ref(props.image);

const setCategory = (category: string) => {
	emit('update:categorySelected', category);
};

const getAppIcon = async () => {
	appIcon.value = await getIconSource(props.image);
};

onMounted(() => {
	getAppIcon();
});
</script>

<template>
  <button
    class="p-2 rounded-corner hover:scale-120"
    @click="setCategory(category)"
:class="[
    category === categorySelected
      ? 'bg-primary/50 animate-[pulse_2s_infinite_ease-in-out] border border-[rgba(127,127,127,0.1)] relative after:content-[\'\'] after:absolute after:-inset-[1px] auto-after-inherit after:rounded-[inherit] after:bg-[linear-gradient(45deg,rgba(127,127,127,0)_0%,rgba(127,127,127,0.1)_50%,rgba(127,127,127,0)_100%)] after:animate-shine'
      : 'bg-transparent'
  ]"
  >
    <img :src="appIcon" :title="description" :alt="category" class="h-12" />
  </button>
</template>

