<script setup lang="ts">
/**
 * El mosaico «Modo avión» (vasak-desktop#180): apaga todas las radios y, al
 * quitarlo, prende sólo las que estaban prendidas. Sin detalle. Sin radios, o
 * sin permiso sobre `/dev/rfkill`, se ve no disponible.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { QuickSettingsTile } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import { useAirplaneMode } from '@/tools/composables/useAirplaneMode';

const { t } = useI18n();
const { available, enabled, busy, failed, hardware, toggle } = useAirplaneMode();

const status = computed(() => {
	if (failed.value) return t('components.ControlCenterTiles.airplaneModeFailed');
	if (enabled.value && hardware.value)
		return t('components.ControlCenterTiles.airplaneModeHardware');
	return enabled.value
		? t('components.ControlCenterTiles.enabled')
		: t('components.ControlCenterTiles.disabled');
});
</script>

<template>
  <QuickSettingsTile
    icon="airplane-mode-symbolic"
    :title="t('components.ControlCenterTiles.airplaneMode')"
    :status="status"
    :active="enabled"
    :loading="busy"
    :unavailable="!available"
    :unavailable-label="t('components.ControlCenterTiles.unavailable')"
    @activate="toggle"
  />
</template>
