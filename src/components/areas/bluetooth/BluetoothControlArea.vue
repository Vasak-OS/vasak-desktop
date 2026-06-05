<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import {
	type AdapterInfo,
	connectDevice,
	disconnectDevice,
	getAvailableDevices,
	getConnectedDevices,
	getDefaultAdapter,
	getDeviceInfo,
	scanForDevices,
	toggleBluetooth,
} from '@vasakgroup/plugin-bluetooth-manager';
import { computed, onMounted, ref } from 'vue';
import { useIcon, useSymbol } from '@/tools/composables/useReactiveIcon';
import BluetoothDeviceCard from '@/components/cards/BluetoothDeviceCard.vue';
import SwitchToggle from '@/components/forms/SwitchToggle.vue';
import { applyBluetoothChange, resolveBluetoothIconName } from '@/tools/bluetooth.controller';
import { useEventListener } from '@/tools/event.listener';
import { logError } from '@/utils/logger';

const connectedDevices = ref<any[]>([]);
const availableDevices = ref<any[]>([]);
const isTogglingBluetooth = ref(false);
const syncIcon = useSymbol(computed(() => 'refreshstructure'));
const defaultAdapter = ref<AdapterInfo | null>(null);
const connectedDevicesCount = ref(0);
const loading = ref(true);
const isScanning = ref(false);
const connectingPath = ref<string | null>(null);

const toggleBT = async () => {
	isTogglingBluetooth.value = true;
	try {
		await toggleBluetooth();
		await refreshDevices();
	} catch (error) {
		logError('Error alternando Bluetooth:', error);
	} finally {
		isTogglingBluetooth.value = false;
	}
};

const isBluetoothOn = computed(() => defaultAdapter.value?.powered);

const handleBluetoothChange = async (event: any) => {
	applyBluetoothChange(event.payload, {
		availableDevices,
		connectedDevices,
		defaultAdapter,
	});
};

const refreshDevices = async () => {
	if (!defaultAdapter.value) return;
	loading.value = true;
	try {
		connectedDevices.value = await getConnectedDevices(defaultAdapter.value.path);
		availableDevices.value = await getAvailableDevices(defaultAdapter.value.path);
		connectedDevicesCount.value = connectedDevices.value.length;
	} catch (e) {
		console.error('Error refreshing devices:', e);
		connectedDevices.value = [];
		availableDevices.value = [];
	}
	loading.value = false;
};

const scanDevices = async () => {
	if (!defaultAdapter.value) return;
	isScanning.value = true;
	try {
		await scanForDevices(defaultAdapter.value.path);
		await refreshDevices();
	} catch (e) {
		console.error('Error scanning for devices:', e);
	}
	isScanning.value = false;
};

const bluetoothIcon = useIcon(computed(() => {
	connectedDevicesCount.value = connectedDevices.value.length;
	return resolveBluetoothIconName(isBluetoothOn.value, connectedDevicesCount.value);
}));

onMounted(async () => {
	defaultAdapter.value = await getDefaultAdapter();
	await refreshDevices();
});

useEventListener('bluetooth-change', handleBluetoothChange);

const connect = async (device: any) => {
	console.log('[BT] Connect clicked for:', device.path, device.name || device.alias);
	connectingPath.value = device.path;
	try {
		await connectDevice(device.path);
		console.log('[BT] connectDevice returned OK, polling for connected state...');
		let connected = false;
		for (let i = 0; i < 30; i++) {
			await new Promise(r => setTimeout(r, 500));
			try {
				const info = await getDeviceInfo(device.path);
				console.log('[BT] Poll', i, 'connected:', info?.connected);
				if (info?.connected) {
					connected = true;
					break;
				}
			} catch (e) {
				console.log('[BT] Poll', i, 'error:', e);
			}
		}
		console.log('[BT] Poll done, connected:', connected);
		await refreshDevices();
	} catch (e) {
		console.error('[BT] connectDevice threw:', e);
		logError('Error connecting to device:', e);
	}
	connectingPath.value = null;
};
const disconnect = async (device: any) => {
	try {
		await disconnectDevice(device.path);
		await refreshDevices();
	} catch (e) {
		console.error('[BT] disconnectDevice threw:', e);
		logError('Error disconnecting device:', e);
	}
};
</script>

<template>
  <div class="flex flex-col h-full">
    <div class="flex items-center mb-4">
      <SwitchToggle
        :is-on="isBluetoothOn || false"
        :disabled="isTogglingBluetooth"
        size="medium"
        active-class="bg-primary"
        inactive-class="bg-gray-400"
        custom-class="mr-2 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary"
        @toggle="toggleBT"
      />
      <img :src="bluetoothIcon" alt="Bluetooth" class="h-8 w-auto mr-3" />
      <span class="font-bold text-2xl flex-1">Bluetooth</span>
      <button
        class="bg-primary text-white rounded-corner px-1 py-0.5 active:bg-primary/80 disabled:cursor-not-allowed disabled:opacity-50"
        @click="scanDevices"
        :disabled="!isBluetoothOn || isScanning"
      >
        <img
          :src="syncIcon"
          alt="Sync Bluetooth"
          class="h-6 w-6"
          :class="{ 'animate-spin': isScanning }"
        />
      </button>
    </div>
    <div v-if="loading" class="text-center px-6 flex-1">Cargando...</div>
    <div v-else class="flex-1 flex gap-4 flex-col">
      <div class="flex-1 flex flex-col overflow-hidden">
        <div class="mb-4 font-semibold text-xl">Disponibles</div>
        <div class="flex-1 overflow-y-auto">
          <div
            v-if="availableDevices.length === 0"
            class="text-tx-muted text-sm px-1.5 text-center"
          >
            No hay dispositivos disponibles
          </div>
          <ul v-else class="list-none p-0 m-0">
            <li v-for="dev in availableDevices" :key="dev.path">
              <BluetoothDeviceCard
                :device="dev"
                action-label="Conectar"
                :is-connecting="connectingPath === dev.path"
                @action="connect(dev)"
              />
            </li>
          </ul>
        </div>
      </div>
      <div class="flex-1 flex flex-col overflow-hidden">
        <div class="mb-4 font-semibold text-xl">Dispositivos conectados</div>
        <div class="flex-1 overflow-y-auto">
          <div
            v-if="connectedDevices.length === 0"
            class="text-tx-muted text-sm px-1.5 text-center"
          >
            Ningún dispositivo conectado
          </div>
          <ul v-else class="list-none p-0 m-0">
            <li v-for="dev in connectedDevices" :key="dev.path">
              <BluetoothDeviceCard
                :device="dev"
                action-label="Desconectar"
                connected
                @action="disconnect(dev)"
              />
            </li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</template>
