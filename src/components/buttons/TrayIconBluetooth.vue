
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import { toggleBluetoothApplet } from '@/services/window.service';
import { useBluetoothState } from '@/tools/bluetooth.controller';
import { logError } from '@/utils/logger';

const { bluetoothIcon, isBluetoothOn, connectedDevicesCount } = useBluetoothState({
	getIcon: getSymbolSource,
});

const toggleBluetooth = async (): Promise<void> => {
	try {
		await toggleBluetoothApplet();
	} catch (error) {
		logError('Error toggling bluetooth applet:', error);
	}
};
</script>

<template>
  <TrayIconButton
    :icon="bluetoothIcon"
    :tooltip="isBluetoothOn ? 'Bluetooth On' : 'Bluetooth Off'"
    alt="Bluetooth Icon"
    :badge="isBluetoothOn && connectedDevicesCount > 0 ? connectedDevicesCount : null"
    :icon-class="{ 'filter brightness-75': !isBluetoothOn }"
    @click="toggleBluetooth"
  />
</template>
