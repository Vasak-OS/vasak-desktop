<script setup lang="ts">
/** El mosaico que abre el tablero de tiempo de pantalla (vasak-desktop#150). */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { openScreenTime } from '@/tools/control-center-actions';
import ControlCenterTile from './ControlCenterTile.vue';

const { t } = useI18n();
</script>

<template>
  <ControlCenterTile
    icon="preferences-system-time"
    :title="t('components.ControlCenterTiles.screenTime')"
    @click="openScreenTime"
  />
</template>
