<script setup lang="ts">
/**
 * El mosaico «Juegos» (vasak-desktop#181): prende y apaga el modo juego, sin
 * detalle. Lo que el modo hace al entrar se elige en Configuración. Si el
 * cambio falla, el mosaico queda en el estado real y la línea de estado lo
 * dice; sin el comando en el escritorio se ve no disponible.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { QuickSettingsTile } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import { useGameMode } from '@/tools/composables/useGameMode';

const { t } = useI18n();
const { available, enabled, busy, failed, toggle } = useGameMode();

const status = computed(() => {
	if (failed.value) return t('components.ControlCenterTiles.gameModeFailed');
	return enabled.value
		? t('components.ControlCenterTiles.enabled')
		: t('components.ControlCenterTiles.disabled');
});
</script>

<template>
  <QuickSettingsTile
    icon="input-gaming-symbolic"
    :title="t('components.ControlCenterTiles.gameMode')"
    :status="status"
    :active="enabled"
    :loading="busy"
    :unavailable="!available"
    :unavailable-label="t('components.ControlCenterTiles.unavailable')"
    @activate="toggle"
  />
</template>
