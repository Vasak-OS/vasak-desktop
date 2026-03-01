<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { computed } from 'vue';
import AppMenuButton from '@/components/buttons/AppMenuButton.vue';

const props = defineProps({
	apps: {
		type: Array,
		required: true,
	},
	filter: {
		type: String,
		required: true,
	},
});

const appsFiltred = computed((): Array<any> => {
	return props.apps.filter(
		(app: any) =>
			app.name.toLowerCase().includes(props.filter) ||
			app.description.toLowerCase().includes(props.filter)
	);
});
</script>

<template>
  <transition-group
    tag="div"
    move-class="transition-transform duration-300 ease-out" enter-active-class="transition-all duration-500 ease-out" leave-active-class="transition-all duration-400 ease-in" enter-from-class="opacity-0 scale-80 translate-y-[20px]" leave-to-class="opacity-0 scale-90 -translate-x-[10px]"
    appear
    class="flex flex-wrap gap-1 p-0.5"
  >
    <AppMenuButton 
      v-for="app in appsFiltred" 
      :key="app.name" 
      :app="app" 
      class="transition-all duration-300 ease-out hover:-translate-y-[2px] hover:scale-[1.02] hover:shadow-[0_4px_12px_rgba(0,0,0,0.15)]"
    />
  </transition-group>
</template>

