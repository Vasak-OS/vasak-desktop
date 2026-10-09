<script setup lang="ts">
/** El mosaico del tema: alterna claro y oscuro, como el botón redondo. */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { QuickSettingsTile } from '@vasakgroup/vue-libvasak';
import { useThemeToggle } from '@/tools/composables/useThemeToggle';

const { t } = useI18n();
const { isDark, isSwitching, themeIcon, toggleTheme } = useThemeToggle();
</script>

<template>
  <QuickSettingsTile
    :icon="themeIcon"
    :title="t('components.ControlCenterTiles.theme')"
    :status="isDark ? t('components.ControlCenterTiles.dark') : t('components.ControlCenterTiles.light')"
    :active="isDark"
    :loading="isSwitching"
    @activate="toggleTheme"
  />
</template>
