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
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import BluetoothDeviceCard from '@/components/cards/BluetoothDeviceCard.vue';
import SwitchToggle from '@/components/forms/SwitchToggle.vue';
import { applyBluetoothChange, resolveBluetoothIconName } from '@/tools/bluetooth.controller';
import { useIcon, useSymbol } from '@/tools/composables/useReactiveIcon';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

const { t } = useI18n();

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

const handleBluetoothChange = async (payload: any) => {
	applyBluetoothChange(payload, {
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

const bluetoothIcon = useIcon(
	computed(() => {
		connectedDevicesCount.value = connectedDevices.value.length;
		return resolveBluetoothIconName(isBluetoothOn.value, connectedDevicesCount.value);
	})
);

onMounted(async () => {
	defaultAdapter.value = await getDefaultAdapter();
	await refreshDevices();
});

useSharedEvent('bluetooth-change', handleBluetoothChange);

const connect = async (device: any) => {
	connectingPath.value = device.path;
	try {
		await connectDevice(device.path);
		for (let i = 0; i < 30; i++) {
			await new Promise((r) => setTimeout(r, 500));
			try {
				const info = await getDeviceInfo(device.path);
				if (info?.connected) break;
			} catch {
				// keep polling
			}
		}
		await refreshDevices();
	} catch (e) {
		logError('Error connecting to device:', e);
	}
	connectingPath.value = null;
};
const disconnect = async (device: any) => {
	try {
		await disconnectDevice(device.path);
		await refreshDevices();
	} catch (e) {
		logError('Error disconnecting device:', e);
	}
};
</script>

<template>
  <div class="flex flex-col h-full">
    <div class="flex items-center mb-4">
      <SwitchToggle :label="t('components.BluetoothControl.toggle')"
        :is-on="isBluetoothOn || false"
        :disabled="isTogglingBluetooth"
        size="medium"
        active-class="bg-primary"
        inactive-class="bg-tx-muted"
        custom-class="mr-2 focus:ring-2 focus:ring-offset-2 focus:ring-primary"
        @toggle="toggleBT"
      />
      <img :src="bluetoothIcon" alt="Bluetooth" class="h-8 w-auto mr-3" />
      <span class="font-bold text-2xl flex-1">Bluetooth</span>
      <button
        class="bg-primary text-white rounded-corner px-1 py-0.5 active:bg-primary/80 disabled:cursor-not-allowed disabled:opacity-50"
        @click="scanDevices"
        :disabled="!isBluetoothOn || isScanning" :aria-label="t('components.BluetoothControlArea.scanAlt')">
        <img
          :src="syncIcon"
          :alt="t('components.BluetoothControlArea.scanAlt')"
          class="h-6 w-6"
          :class="{ 'animate-spin': isScanning }"
        />
      </button>
    </div>
    <div v-if="loading" class="text-center px-6 flex-1">{{ t('common.loading') }}</div>
    <div v-else class="flex-1 flex gap-4 flex-col">
      <div class="flex-1 flex flex-col overflow-hidden">
        <div class="mb-4 font-semibold text-xl">{{ t('components.BluetoothControlArea.available') }}</div>
        <div class="flex-1 overflow-y-auto">
          <div
            v-if="availableDevices.length === 0"
            class="text-tx-muted text-sm px-1.5 text-center"
          >
            {{ t('components.BluetoothControlArea.noneAvailable') }}
          </div>
          <ul v-else class="list-none p-0 m-0">
            <li v-for="dev in availableDevices" :key="dev.path">
              <BluetoothDeviceCard
                :device="dev"
                :action-label="t('components.BluetoothControlArea.connect')"
                :is-connecting="connectingPath === dev.path"
                @action="connect(dev)"
              />
            </li>
          </ul>
        </div>
      </div>
      <div class="flex-1 flex flex-col overflow-hidden">
        <div class="mb-4 font-semibold text-xl">{{ t('components.BluetoothControlArea.connectedDevices') }}</div>
        <div class="flex-1 overflow-y-auto">
          <div
            v-if="connectedDevices.length === 0"
            class="text-tx-muted text-sm px-1.5 text-center"
          >
            {{ t('components.BluetoothControlArea.noneConnected') }}
          </div>
          <ul v-else class="list-none p-0 m-0">
            <li v-for="dev in connectedDevices" :key="dev.path">
              <BluetoothDeviceCard
                :device="dev"
                :action-label="t('components.BluetoothControlArea.disconnect')"
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
