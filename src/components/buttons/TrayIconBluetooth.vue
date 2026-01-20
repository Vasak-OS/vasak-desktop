<script lang="ts" setup>
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { invoke } from '@tauri-apps/api/core';
import { useBluetoothState } from '@/composables/useBluetoothState';

const {
	bluetoothIcon,
	isBluetoothOn,
	connectedDevicesCount,
} = useBluetoothState({
	getIcon: getSymbolSource,
});

const toggleBluetooth = async (): Promise<void> => {
	try {
		await invoke('toggle_bluetooth_applet');
	} catch (error) {
		console.error('Error toggling bluetooth applet:', error);
	}
};
</script>
<template>
  <div
    class="p-1 relative hover:bg-vsk-primary/30 rounded-vsk"
    :title="isBluetoothOn ? 'Bluetooth On' : 'Bluetooth Off'"
    @click="toggleBluetooth"
  >
    <img
      :src="bluetoothIcon"
      alt="Bluetooth Icon"
      class="m-auto h-5.5 w-auto transition-all duration-300"
      :class="{
        'filter brightness-75': !isBluetoothOn,
      }"
    />
    <div
      v-if="isBluetoothOn && connectedDevicesCount > 0"
      class="absolute bottom-1 right-1 bg-vsk-primary text-white text-xs rounded-full w-4 h-4 flex items-center justify-center font-bold animate-bounce"
    >
      {{ connectedDevicesCount }}
    </div>
  </div>
</template>
