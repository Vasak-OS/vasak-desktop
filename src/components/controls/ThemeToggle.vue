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
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ToggleControl, type ToggleIndicator } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import { useThemeToggle } from '@/tools/composables/useThemeToggle';

const { t } = useI18n();

const { isDark, isSwitching, themeIcon, toggleTheme } = useThemeToggle();

/**
 * El sol y la luna los dibuja el icono del tema (`themeIcon`), que ya dice en
 * qué modo está. El punto de la esquina es el `indicator` de `ToggleControl`
 * (vue-libvasak 2.2.0) con dos tonos del esquema que se distinguen entre sí:
 * el aviso para el sol y el acento para la luna. Los colores salen del
 * esquema, sin excepciones (30/09/2026).
 */
const indicator = computed<ToggleIndicator>(() => ({ tone: isDark.value ? 'accent' : 'warning' }));
</script>
