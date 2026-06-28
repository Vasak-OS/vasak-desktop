<template>
  <div class="relative inline-block theme-toggle-wrapper" :class="{ 'theme-switching': isSwitching }">
    <!-- Sun/Moon indicator -->
    <div class="absolute top-1 right-1 w-3 h-3 rounded-full transition-all duration-500 z-20" :class="{
      'bg-yellow-400 animate-pulse': !(configStore?.config as any)?.style?.darkmode,
      'bg-blue-400 animate-pulse': (configStore?.config as any)?.style?.darkmode,
    }"></div>

    <!-- Background gradient effect -->
    <div
      class="absolute inset-0 rounded-corner transition-all duration-500 pointer-events-none opacity-0 group-hover:opacity-100 group-hover:opacity-100 transition-opacity !opacity-100"
      :class="{
        'bg-lineal-to-br from-orange-400/20 to-yellow-400/20':
          !(configStore?.config as any)?.style?.darkmode,
        'bg-lineal-to-br from-purple-500/20 to-blue-600/20':
          (configStore?.config as any)?.style?.darkmode,
      }"></div>

    <ToggleControl :icon="themeIcon" :alt="(configStore?.config as any)?.style?.darkmode
        ? 'Toggle light theme'
        : 'Toggle dark theme'
      " :tooltip="(configStore?.config as any)?.style?.darkmode
          ? 'Toggle light theme'
          : 'Toggle dark theme'
        " :is-active="true" :is-loading="isSwitching" :custom-class="{
        'h-[70px] w-[70px] p-2': true,
        'ring-2 ring-primary': true,
      }" :icon-class="{
        'w-[50px] h-[50px]': true,
        'filter brightness-110': !(configStore?.config as any)?.style?.darkmode,
      }" @click="toggleTheme" />
  </div>
</template>

<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { setDarkMode, useConfigStore } from '@vasakgroup/plugin-config-manager';
import type { Store } from 'pinia';
import { computed, onMounted, type Ref, ref } from 'vue';
import { useReactiveSymbol } from '@/tools/composables/useReactiveIcon';
import { logError } from '@/utils/logger';
import ToggleControl from '../forms/ToggleControl.vue';

const configStore = ref<any>(null);
const isSwitching: Ref<boolean> = ref(false);

const themeIcon = useReactiveSymbol(
	computed(() =>
		configStore.value?.config?.style?.darkmode ? 'weather-clear' : 'weather-clear-night'
	)
);

onMounted(() => {
	configStore.value = useConfigStore() as Store<
		'config',
		{ config: any; loadConfig: () => Promise<void> }
	>;
});

const toggleTheme = async () => {
	if (isSwitching.value || !configStore.value) return;

	isSwitching.value = true;
	try {
		const currentDark = !!configStore.value?.config?.style?.darkmode;
		// Toggle immediately so the UI responds instantly
		document.documentElement.classList.toggle('dark', !currentDark);
		await setDarkMode(!currentDark);
	} catch (error) {
		// Revert on error
		const currentDark = !!configStore.value?.config?.style?.darkmode;
		document.documentElement.classList.toggle('dark', currentDark);
		logError('Error toggling system theme:', error);
	} finally {
		setTimeout(() => {
			isSwitching.value = false;
		}, 800);
	}
};
</script>

