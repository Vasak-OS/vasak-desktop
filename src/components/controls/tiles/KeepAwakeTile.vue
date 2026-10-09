<script setup lang="ts">
/**
 * El mosaico «Mantener despierto» (vasak-desktop#179): mientras está encendido
 * no se bloquea la pantalla ni se suspende por inactividad. Sin detalle. Si el
 * cambio falla, queda en el estado real y la línea de estado lo dice; sin
 * logind se ve no disponible.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { QuickSettingsTile } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import { useKeepAwake } from '@/tools/composables/useKeepAwake';

const { t } = useI18n();
const { available, enabled, busy, failed, toggle } = useKeepAwake();

const status = computed(() => {
	if (failed.value) return t('components.ControlCenterTiles.keepAwakeFailed');
	return enabled.value
		? t('components.ControlCenterTiles.enabled')
		: t('components.ControlCenterTiles.disabled');
});
</script>

<template>
  <QuickSettingsTile
    :icon="enabled ? 'caffeine-cup-full-symbolic' : 'caffeine-cup-empty-symbolic'"
    :title="t('components.ControlCenterTiles.keepAwake')"
    :status="status"
    :active="enabled"
    :loading="busy"
    :unavailable="!available"
    :unavailable-label="t('components.ControlCenterTiles.unavailable')"
    @activate="toggle"
  />
</template>
