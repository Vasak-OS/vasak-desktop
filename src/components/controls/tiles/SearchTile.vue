<script setup lang="ts">
/** El mosaico que abre el lanzador (`vasak-prism`). */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { openSearch } from '@/tools/control-center-actions';
import ControlCenterTile from './ControlCenterTile.vue';

const { t } = useI18n();
</script>

<template>
  <ControlCenterTile
    icon="system-search"
    :title="t('components.ControlCenterTiles.search')"
    @click="openSearch"
  />
</template>
