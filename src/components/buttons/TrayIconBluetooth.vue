<script lang="ts" setup>
import { ref, computed, onMounted, onUnmounted, Ref } from 'vue';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import {
	getDefaultAdapter,
	AdapterInfo,
	getConnectedDevicesCount,
} from '@vasakgroup/plugin-bluetooth-manager';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import {
	applyBluetoothChange,
	resolveBluetoothIconName,
} from '@/tools/bluetooth.controller';

const connectedDevices: Ref<any[]> = ref([]);
const availableDevices: Ref<any[]> = ref([]);
const bluetoothIcon: Ref<string> = ref('');
const defaultAdapter = ref<AdapterInfo | null>(null);
const connectedDevicesCount: Ref<number> = ref(0);
let unlistenBluetooth: Ref<(() => void) | null> = ref(null);

const isBluetoothOn = computed(() => {
	return defaultAdapter.value?.powered;
});

const handleBluetoothChange = async (event: any) => {
	applyBluetoothChange(event.payload, {
		availableDevices,
		connectedDevices,
		defaultAdapter,
	});
	await getBluetoothIcon();
};

onMounted(async () => {
	defaultAdapter.value = await getDefaultAdapter();
	await getBluetoothIcon();
	connectedDevicesCount.value = await getConnectedDevicesCount(
    defaultAdapter.value?.path as string
	);
	unlistenBluetooth.value = await listen(
		'bluetooth-change',
		handleBluetoothChange
	);
});

onUnmounted(() => {
	if (unlistenBluetooth.value) {
		unlistenBluetooth.value();
	}
});

const toggleBluetooth = async () => {
	try {
		await invoke('toggle_bluetooth_applet');
	} catch (error) {
		console.error('Error toggling bluetooth applet:', error);
	}
};

const getBluetoothIcon = async () => {
	try {
		connectedDevicesCount.value = await getConnectedDevicesCount(
      defaultAdapter.value?.path as string
		);
		const iconName = resolveBluetoothIconName(
			isBluetoothOn.value,
			connectedDevicesCount.value
		);

		bluetoothIcon.value = await getSymbolSource(iconName);
	} catch (error) {
		console.error('Error loading bluetooth icon:', error);
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
      class="m-auto h-[22px] w-auto transition-all duration-300"
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
