<template>
  <button
    @click="toggleBT"
    class="p-2 rounded-vsk background hover:opacity-50 transition-all duration-300 h-17.5 w-17.5 group relative overflow-hidden hover:scale-105 hover:shadow-lg active:scale-95"
    :class="{
      'animate-pulse': isTogglingBluetooth,
      'ring-2 ring-vsk-primary/50': isBluetoothOn,
      'opacity-60': !isBluetoothOn,
    }"
    :disabled="isTogglingBluetooth"
  >
    <!-- Background glow effect -->
    <div
      class="absolute inset-0 rounded-vsk bg-lineal-to-br from-blue-500/20 to-indigo-500/20 opacity-0 group-hover:opacity-100 transition-opacity duration-300"
    ></div>

    <!-- Status indicator -->
    <div
      class="absolute top-1 right-1 w-3 h-3 rounded-full transition-all duration-300"
      :class="{
        'bg-blue-400 animate-pulse': isBluetoothOn && connectedDevicesCount > 0,
        'bg-blue-400': isBluetoothOn && connectedDevicesCount === 0,
        'bg-gray-400': !isBluetoothOn,
      }"
    ></div>

    <!-- Connected devices count -->
    <div
      v-if="isBluetoothOn && connectedDevicesCount > 0"
      class="absolute bottom-1 right-1 bg-vsk-primary text-white text-xs rounded-full w-4 h-4 flex items-center justify-center font-bold animate-bounce"
    >
      {{ connectedDevicesCount }}
    </div>

    <!-- Bluetooth waves animation when enabled -->
    <div
      v-if="isBluetoothOn"
      class="absolute inset-0 flex items-center justify-center"
    >
      <div
        class="absolute w-8 h-8 border-2 border-vsk-primary/30 rounded-full animate-ping"
      ></div>
      <div
        class="absolute w-12 h-12 border-2 border-vsk-primary/20 rounded-full animate-ping"
        style="animation-delay: 0.5s"
      ></div>
    </div>

    <img
      :src="bluetoothIcon"
      alt="Bluetooth Icon"
      class="m-auto w-12.5 h-12.5 transition-all duration-300 group-hover:scale-110 relative z-10"
      :class="{
        'animate-spin': isTogglingBluetooth,
        'filter brightness-75': !isBluetoothOn,
        'drop-shadow-lg': isBluetoothOn,
      }"
    />
  </button>
</template>

<script setup lang="ts">
import { ref, Ref } from 'vue';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import {
	toggleBluetooth,
} from '@vasakgroup/plugin-bluetooth-manager';
import { useBluetoothState } from '@/tools/bluetooth.controller';

const isTogglingBluetooth: Ref<boolean> = ref(false);

const {
	bluetoothIcon,
	isBluetoothOn,
	connectedDevicesCount,
} = useBluetoothState({
	getIcon: getIconSource,
});

const toggleBT = async (): Promise<void> => {
	isTogglingBluetooth.value = true;
	try {
		await toggleBluetooth();
	} catch (error) {
		console.error('Error toggling Bluetooth:', error);
	} finally {
		isTogglingBluetooth.value = false;
	}
};
</script>
