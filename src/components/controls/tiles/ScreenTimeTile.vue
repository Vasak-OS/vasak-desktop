<script setup lang="ts">
/** El mosaico que abre el tablero de tiempo de pantalla (vasak-desktop#150). */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { QuickSettingsTile } from '@vasakgroup/vue-libvasak';
import { openScreenTime } from '@/tools/control-center-actions';

const { t } = useI18n();
</script>

<template>
  <QuickSettingsTile
    icon="preferences-system-time"
    :title="t('components.ControlCenterTiles.screenTime')"
    @activate="openScreenTime"
  />
</template>
