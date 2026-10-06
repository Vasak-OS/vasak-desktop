<script setup lang="ts">
/** El mosaico del tema: alterna claro y oscuro, como el botón redondo. */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { useThemeToggle } from '@/tools/composables/useThemeToggle';
import ControlCenterTile from './ControlCenterTile.vue';

const { t } = useI18n();
const { isDark, isSwitching, themeIcon, toggleTheme } = useThemeToggle();
</script>

<template>
  <ControlCenterTile
    :icon="themeIcon"
    :title="t('components.ControlCenterTiles.theme')"
    :description="isDark ? t('components.ControlCenterTiles.dark') : t('components.ControlCenterTiles.light')"
    :pressed="isDark"
    :disabled="isSwitching"
    @click="toggleTheme"
  />
</template>
