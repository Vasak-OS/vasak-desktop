<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { useIcon } from '@/tools/composables/useReactiveIcon';
import { toggleSearch } from '@/services/window.service';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const iconSrc = useIcon('search');

const openSearch = async () => {
	try {
		await toggleSearch();
	} catch (error) {
		logError('Error opening search:', error);
	}
};
</script>

<template>
  <button
    @click="openSearch"
    class="p-2 rounded-corner bg-ui-bg/80 transition-all duration-500 h-17 w-17 group relative overflow-hidden hover:scale-105 active:scale-95 ring-2 ring-primary"
    :title="t('components.SearchButtonControl.openSearch')"
  >
    <!-- Overlay decorativo como ThemeToggle -->
    <div
      class="absolute inset-0 rounded-corner transition-all duration-500"
      :class="'bg-lineal-to-br from-primary to-primary-secondary'"
      style="opacity: 0"
    ></div>

    <img
      :src="iconSrc"
      :alt="t('components.SearchButtonControl.searchAlt')"
      class="m-auto w-12 h-12 transition-all duration-500 group-hover:scale-110 relative z-10 drop-shadow-lg group-hover:drop-shadow-xl"
    />
  </button>
</template>
