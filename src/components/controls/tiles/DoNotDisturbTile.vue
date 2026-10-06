<script setup lang="ts">
/**
 * El mosaico de «No molestar» (vasak-desktop#177): lo prende y lo apaga, sin
 * detalle. Sin demonio de notificaciones que lo entienda se ve no disponible.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { QuickSettingsTile } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import { useDoNotDisturb } from '@/tools/composables/useDoNotDisturb';

const { t } = useI18n();
const { available, enabled, busy, toggle } = useDoNotDisturb();

const status = computed(() =>
	enabled.value
		? t('components.ControlCenterTiles.doNotDisturbOn')
		: t('components.ControlCenterTiles.disabled')
);
</script>

<template>
  <QuickSettingsTile
    icon="notifications-disabled-symbolic"
    :title="t('components.ControlCenterTiles.doNotDisturb')"
    :status="status"
    :active="enabled"
    :loading="busy"
    :unavailable="!available"
    :unavailable-label="t('components.ControlCenterTiles.unavailable')"
    @activate="toggle"
  />
</template>
