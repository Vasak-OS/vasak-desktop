<template>
  <ToggleControl
    class="theme-transition"
    :name="themeIcon"
    type="symbol"
    :label="isDark ? t('components.ThemeToggle.toLight') : t('components.ThemeToggle.toDark')"
    :pressed="isDark"
    :is-active="true"
    :is-loading="isSwitching"
    :indicator="indicator"
    @click="toggleTheme"
  />
</template>

<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { setDarkMode, useConfigStore } from '@vasakgroup/plugin-config-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ToggleControl, type ToggleIndicator } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, type Ref, ref } from 'vue';
import { cancelRunningThemeTransitions } from '@/tools/theme.utils';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const configStore = ref<any>(null);
const isSwitching: Ref<boolean> = ref(false);

const isDark = computed(() => Boolean(configStore.value?.config?.style?.darkmode));

const themeIcon = computed(() => (isDark.value ? 'weather-clear' : 'weather-clear-night'));

/**
 * El sol y la luna los dibuja el icono del tema (`themeIcon`), que ya dice en
 * qué modo está. El punto de la esquina es el `indicator` de `ToggleControl`
 * (vue-libvasak 2.2.0) con dos tonos del esquema que se distinguen entre sí:
 * el aviso para el sol y el acento para la luna. Los amarillos, naranjas,
 * azules y violetas que hubo alrededor eran de la paleta de Tailwind y se
 * fueron (30/09/2026: los colores salen del esquema, sin excepciones).
 */
const indicator = computed<ToggleIndicator>(() => ({ tone: isDark.value ? 'accent' : 'warning' }));

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

