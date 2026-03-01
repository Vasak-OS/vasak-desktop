<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { onMounted, type Ref, ref } from 'vue';
import { toggleSearch } from '@/services/window.service';
import { logError } from '@/utils/logger';

const iconSrc: Ref<string> = ref('');

const openSearch = async () => {
	try {
		await toggleSearch();
	} catch (error) {
		logError('Error opening search:', error);
	}
};

onMounted(async () => {
	iconSrc.value = await getIconSource('search');
});
</script>

<template>
  <button
    @click="openSearch"
    class="p-2 rounded-corner background transition-all duration-500 h-17 w-17 group relative overflow-hidden hover:scale-105 active:scale-95 ring-2 ring-primary dark:ring-primary-dark"
    title="Open Global Search"
  >
    <!-- Overlay decorativo como ThemeToggle -->
    <div
      class="absolute inset-0 rounded-corner transition-all duration-500"
      :class="'bg-lineal-to-br from-primary dark:from-primary-dark to-primary-secondary dark:to-secondary-dark'"
      style="opacity: 0"
    ></div>

    <img
      :src="iconSrc"
      alt="Search"
      class="m-auto w-12 h-12 transition-all duration-500 group-hover:scale-110 relative z-10 drop-shadow-lg group-hover:drop-shadow-xl"
    />
  </button>
</template>
