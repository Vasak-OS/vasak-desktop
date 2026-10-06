<script setup lang="ts">
/**
 * El mosaico de Bluetooth: dice si está encendido y cuántos dispositivos hay
 * conectados, y abre su detalle dentro del bloque.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed } from 'vue';
import { useBluetoothState } from '@/tools/bluetooth.controller';
import ControlCenterTile from './ControlCenterTile.vue';

const emit = defineEmits<{ open: [] }>();
const { t } = useI18n();
const { isBluetoothOn, connectedDevicesCount } = useBluetoothState({ getIcon: async () => '' });

const icon = computed(() => {
	if (!isBluetoothOn.value) return 'bluetooth-disabled';
	return connectedDevicesCount.value > 0 ? 'bluetooth-active' : 'bluetooth';
});

const description = computed(() => {
	if (!isBluetoothOn.value) return t('components.ControlCenterTiles.disabled');
	const count = connectedDevicesCount.value;
	if (count === 0) return t('components.ControlCenterTiles.enabled');
	return t(
		count === 1
			? 'components.ControlCenterTiles.devicesOne'
			: 'components.ControlCenterTiles.devicesOther'
	).replace('{0}', String(count));
});
</script>

<template>
  <ControlCenterTile
    :icon="icon"
    :title="t('components.ControlCenterTiles.bluetooth')"
    :description="description"
    has-detail
    @click="emit('open')"
  />
</template>
