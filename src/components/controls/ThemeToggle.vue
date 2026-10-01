<template>
  <div class="theme-transition relative inline-block theme-toggle-wrapper" :class="{ 'theme-switching': isSwitching }">
    <!-- El punto del sol o de la luna, con colores del esquema: el aviso del
         esquema para el sol y el secundario para la luna. El degradado de
         fondo se fue: era decoración, y sus naranjas y violetas eran de la
         paleta de Tailwind, no del esquema que eligió la persona. -->
    <div class="absolute top-1 right-1 w-3 h-3 rounded-corner-full transition-colors duration-300 z-20" :class="{
      'bg-status-warning': !(configStore?.config as any)?.style?.darkmode,
      'bg-secondary': (configStore?.config as any)?.style?.darkmode,
    }"></div>

    <ToggleControl :name="themeIcon" type="symbol" :label="(configStore?.config as any)?.style?.darkmode
        ? t('components.ThemeToggle.toLight')
        : t('components.ThemeToggle.toDark')
      " :pressed="Boolean((configStore?.config as any)?.style?.darkmode)" :is-active="true" :is-loading="isSwitching" @click="toggleTheme" />
  </div>
</template>

<!-- El sol y la luna los dibuja el icono del tema (`themeIcon`), que ya dice
     en qué modo está. Los amarillos, naranjas, azules y violetas que había
     alrededor eran de la paleta de Tailwind y se fueron (decisión del
     30/09/2026: los colores salen del esquema, sin excepciones); el punto que
     queda usa dos colores del esquema que se distinguen entre sí. -->
<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { setDarkMode, useConfigStore } from '@vasakgroup/plugin-config-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ToggleControl } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, type Ref, ref } from 'vue';
import { cancelRunningThemeTransitions } from '@/tools/theme.utils';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const configStore = ref<any>(null);
const isSwitching: Ref<boolean> = ref(false);

const themeIcon = computed(() =>
	configStore.value?.config?.style?.darkmode ? 'weather-clear' : 'weather-clear-night'
);

onMounted(() => {
	configStore.value = useConfigStore();
});

const toggleTheme = async () => {
	if (isSwitching.value || !configStore.value) return;

	isSwitching.value = true;
	try {
		const currentDark = !!configStore.value?.config?.style?.darkmode;
		// Cancel any in-progress theme transitions before applying new values
		cancelRunningThemeTransitions();
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

