<script setup lang="ts">
import { ref, Ref } from 'vue';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import {
	toggleBluetooth,
} from '@vasakgroup/plugin-bluetooth-manager';
import { useBluetoothState } from '@/tools/bluetooth.controller';
import { ToggleControl } from '@vasakgroup/vue-libvasak';

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
  <div class="relative inline-block">
    <!-- Indicador de estado -->
    <div
      class="absolute top-1 right-1 w-3 h-3 rounded-full transition-all duration-300"
      :class="{
        'bg-blue-400 animate-pulse': isBluetoothOn && connectedDevicesCount > 0,
        'bg-blue-400': isBluetoothOn && connectedDevicesCount === 0,
        'bg-gray-400': !isBluetoothOn,
      }"
    ></div>

    <!-- Badge de dispositivos conectados -->
    <div
      v-if="connectedDevicesCount > 0"
      class="absolute bottom-1 right-1 bg-vsk-primary text-white text-xs rounded-full w-4 h-4 flex items-center justify-center font-bold"
    >
      {{ connectedDevicesCount }}
    </div>

    <ToggleControl
      :icon="bluetoothIcon"
      alt="Bluetooth Icon"
      tooltip="Toggle Bluetooth"
      :is-active="isBluetoothOn"
      :is-loading="isTogglingBluetooth"
      :custom-class="{
        'ring-2 ring-blue-400/50': isBluetoothOn,
      }"
      @click="toggleBT"
    />
  </div>
</template>
