<script setup lang="ts">
/** El mosaico que abre el lanzador (`vasak-prism`). */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { QuickSettingsTile } from '@vasakgroup/vue-libvasak';
import { openSearch } from '@/tools/control-center-actions';

const { t } = useI18n();
</script>

<template>
  <QuickSettingsTile
    icon="system-search"
    :title="t('components.ControlCenterTiles.search')"
    @activate="openSearch"
  />
</template>
