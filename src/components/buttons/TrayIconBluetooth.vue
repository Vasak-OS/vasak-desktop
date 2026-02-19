
<script lang="ts" setup>
import { invoke } from '@tauri-apps/api/core';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { TrayIconButton } from '@vasakgroup/vue-libvasak';
import { useBluetoothState } from '@/tools/bluetooth.controller';
import { logError } from '@/utils/logger';

const { bluetoothIcon, isBluetoothOn, connectedDevicesCount } = useBluetoothState({
	getIcon: getSymbolSource,
});

const toggleBluetooth = async (): Promise<void> => {
	try {
		await invoke('toggle_bluetooth_applet');
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
