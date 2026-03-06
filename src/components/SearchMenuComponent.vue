<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
const vFocus = {
	mounted: (el: HTMLElement) => el.focus(),
};

defineProps({
	filter: {
		type: String,
		required: true,
	},
});

const emit = defineEmits(['update:filter']);
</script>

<template>
  <input
    type="text"
    class="form-control border border-ui-border rounded-corner grow bg-ui-bg/80 shadow-none focus:outline-none focus:ring-0 p-2"
    placeholder="Search"
    aria-label="Search"
    id="search"
    v-focus
    :value="filter"
    @input="emit('update:filter', ($event.target as any).value)"
  />
</template>

