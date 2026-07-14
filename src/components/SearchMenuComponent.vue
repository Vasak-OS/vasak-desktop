<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
const vFocus = {
	mounted: (el: HTMLElement) => setTimeout(() => el.focus(), 50),
};

defineProps({
	filter: {
		type: String,
		required: true,
	},
	disabled: {
		type: Boolean,
		default: false,
	},
});

const emit = defineEmits(['update:filter']);
</script>

<template>
  <input
    type="text"
    class="form-control border border-ui-border rounded-corner grow bg-ui-bg/80 shadow-none focus:outline-none focus:ring-0 p-2 disabled:opacity-50 disabled:cursor-not-allowed"
    placeholder="Search"
    aria-label="Search"
    id="search"
    v-focus
    :value="filter"
    :disabled="disabled"
    @input="emit('update:filter', ($event.target as any).value)"
  />
</template>

