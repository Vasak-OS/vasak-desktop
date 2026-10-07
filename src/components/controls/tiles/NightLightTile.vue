<script setup lang="ts">
/**
 * El mosaico «Luz nocturna» (vasak-desktop#178): el cuerpo la prende y la
 * apaga, y la flecha abre el detalle dentro del bloque (temperatura y
 * horario). Sin `wlsunset` instalado se ve no disponible.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { QuickSettingsTile } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import { useNightLight } from '@/tools/composables/useNightLight';

const emit = defineEmits<{ open: [] }>();
const { t } = useI18n();
const { available, enabled, busy, failed, toggle } = useNightLight();

const status = computed(() => {
	if (failed.value) return t('components.ControlCenterTiles.nightLightFailed');
	return enabled.value
		? t('components.ControlCenterTiles.enabled')
		: t('components.ControlCenterTiles.disabled');
});
</script>

<template>
  <QuickSettingsTile
    :icon="enabled ? 'night-light-symbolic' : 'night-light-disabled-symbolic'"
    :title="t('components.ControlCenterTiles.nightLight')"
    :status="status"
    :active="enabled"
    :loading="busy"
    :unavailable="!available"
    :unavailable-label="t('components.ControlCenterTiles.unavailable')"
    detail
    @activate="toggle"
    @detail="emit('open')"
  />
</template>
