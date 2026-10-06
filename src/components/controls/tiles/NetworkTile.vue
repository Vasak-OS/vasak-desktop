<script setup lang="ts">
/**
 * El mosaico de la red: dice a qué está conectado y abre su detalle dentro del
 * bloque (el mismo panel que el applet de red, sin su botón de cerrar).
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted } from 'vue';
import { useNetworkState } from '@/tools/composables/useNetworkState';
import ControlCenterTile from './ControlCenterTile.vue';

const emit = defineEmits<{ open: [] }>();
const { t } = useI18n();
const { networkState, networkIconName, getCurrentNetwork } = useNetworkState();

const description = computed(() =>
	networkState.value.is_connected
		? String(networkState.value.ssid || networkState.value.name)
		: t('components.ControlCenterTiles.disconnected')
);

onMounted(() => {
	void getCurrentNetwork();
});
</script>

<template>
  <ControlCenterTile
    :icon="networkIconName"
    :title="t('components.ControlCenterTiles.network')"
    :description="description"
    has-detail
    @click="emit('open')"
  />
</template>
