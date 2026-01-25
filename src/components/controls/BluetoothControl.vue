<script setup lang="ts">
import { ref, Ref } from 'vue';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import {
	toggleBluetooth,
} from '@vasakgroup/plugin-bluetooth-manager';
import { useBluetoothState } from '@/tools/bluetooth.controller';
import ToggleControl from '@/components/base/ToggleControl.vue';

const isTogglingBluetooth: Ref<boolean> = ref(false);

const {
	bluetoothIcon,
	isBluetoothOn,
	connectedDevicesCount,
} = useBluetoothState({
	getIcon: getIconSource,
});

const toggleBT = async (): Promise<void> => {
	try {
		isTogglingBluetooth.value = true;
		await toggleBluetooth();
	} catch (error) {
		console.error('[Bluetooth Control Error] Error toggling bluetooth:', error);
	} finally {
		isTogglingBluetooth.value = false;
	}
};
</script>

<template>
  <ToggleControl
    :icon="bluetoothIcon"
    alt="Bluetooth Icon"
    tooltip="Toggle Bluetooth"
    :is-active="isBluetoothOn"
    :is-loading="isTogglingBluetooth"
    :badge="connectedDevicesCount > 0 ? connectedDevicesCount : null"
    :show-waves="true"
    :status-indicator-class="{
      'bg-blue-400 animate-pulse': isBluetoothOn && connectedDevicesCount > 0,
      'bg-blue-400': isBluetoothOn && connectedDevicesCount === 0,
      'bg-gray-400': !isBluetoothOn,
    }"
    @click="toggleBT"
  />
</template>
