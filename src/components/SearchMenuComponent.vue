<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';

const { t } = useI18n();

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
    class="form-control border border-ui-border rounded-corner grow bg-ui-bg/80 shadow-none focus:ring-0 p-2 disabled:opacity-50 disabled:cursor-not-allowed"
    :placeholder="t('components.SearchMenuComponent.placeholder')"
    :aria-label="t('components.SearchMenuComponent.placeholder')"
    id="search"
    v-focus
    :value="filter"
    :disabled="disabled"
    @input="emit('update:filter', ($event.target as any).value)"
  />
</template>

