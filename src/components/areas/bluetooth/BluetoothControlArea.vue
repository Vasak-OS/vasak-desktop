<script lang="ts" setup>
import { ref, computed, onMounted, onUnmounted, Ref } from "vue";
import { getIconSource } from "@vasakgroup/plugin-vicons";
import {
  getDefaultAdapter,
  toggleBluetooth,
  AdapterInfo,
  connectDevice,
  disconnectDevice,
  getConnectedDevices,
  getAvailableDevices,
  scanForDevices,
} from "@vasakgroup/plugin-bluetooth-manager";
import { listen } from "@tauri-apps/api/event";
import BluetoothDeviceCard from "@/components/cards/BluetoothDeviceCard.vue";

const connectedDevices: Ref<any[]> = ref([]);
const availableDevices: Ref<any[]> = ref([]);
const isTogglingBluetooth: Ref<boolean> = ref(false);
const bluetoothIcon: Ref<string> = ref("");
const syncIcon: Ref<string> = ref("");
const defaultAdapter = ref<AdapterInfo | null>(null);
const connectedDevicesCount: Ref<number> = ref(0);
const loading: Ref<boolean> = ref(true);
const isScanning = ref(false);

let unlistenBluetooth: Ref<Function | null> = ref(null);

const toggleBT = async () => {
  isTogglingBluetooth.value = true;
  try {
    await toggleBluetooth();
    await refreshDevices();
  } catch (error) {
    console.error("Error toggling Bluetooth:", error);
  } finally {
    isTogglingBluetooth.value = false;
  }
};

const isBluetoothOn = computed(() => defaultAdapter.value?.powered);

// Helper: buscar dispositivo por path
const findDeviceIndex = (devices: any[], path: string) => {
  return devices.findIndex((d) => d.path === path);
};

const deviceExists = (devices: any[], path: string) => {
  return devices.some((d) => d.path === path);
};

// Helper: añadir dispositivo si no existe
const addDeviceIfNotExists = (devices: Ref<any[]>, device: any) => {
  if (!deviceExists(devices.value, device.path)) {
    devices.value.push(device);
  }
};

// Helper: mover dispositivo entre listas
const moveDevice = (from: Ref<any[]>, to: Ref<any[]>, device: any) => {
  const index = findDeviceIndex(from.value, device.path);
  if (index !== -1) {
    from.value.splice(index, 1);
    addDeviceIfNotExists(to, device);
  }
};

// Handler: cambio de propiedad del adaptador
const handleAdapterPropertyChanged = (data: any) => {
  if (defaultAdapter.value && data.path === defaultAdapter.value.path) {
    defaultAdapter.value = data;
  }
};

// Handler: dispositivo añadido
const handleDeviceAdded = (data: any) => {
  addDeviceIfNotExists(availableDevices, data);
};

// Handler: dispositivo removido
const handleDeviceRemoved = (data: any) => {
  availableDevices.value = availableDevices.value.filter(
    (d) => d.path !== data.path
  );
  connectedDevices.value = connectedDevices.value.filter(
    (d) => d.path !== data.path
  );
};

// Handler: actualizar dispositivo en lista disponibles
const updateDeviceInAvailable = (deviceIndex: number, data: any) => {
  if (data.connected) {
    moveDevice(availableDevices, connectedDevices, data);
  } else {
    availableDevices.value[deviceIndex] = data;
  }
};

// Handler: actualizar dispositivo en lista conectados
const updateDeviceInConnected = (connectedIndex: number, data: any) => {
  if (data.connected) {
    connectedDevices.value[connectedIndex] = data;
  } else {
    moveDevice(connectedDevices, availableDevices, data);
  }
};

// Handler: dispositivo conectado o propiedad cambiada
const handleDeviceUpdate = (data: any) => {
  const deviceIndex = findDeviceIndex(availableDevices.value, data.path);

  if (deviceIndex !== -1) {
    updateDeviceInAvailable(deviceIndex, data);
  } else {
    const connectedIndex = findDeviceIndex(connectedDevices.value, data.path);
    if (connectedIndex !== -1) {
      updateDeviceInConnected(connectedIndex, data);
    }
  }
};

// Handler: dispositivo desconectado
const handleDeviceDisconnected = (data: any) => {
  moveDevice(connectedDevices, availableDevices, data);
};

const handleBluetoothChange = async (event: any) => {
  const { change_type, data } = event.payload;

  const handlers: Record<string, (data: any) => void> = {
    "adapter-property-changed": handleAdapterPropertyChanged,
    "device-added": handleDeviceAdded,
    "device-removed": handleDeviceRemoved,
    "device-connected": handleDeviceUpdate,
    "device-property-changed": handleDeviceUpdate,
    "device-disconnected": handleDeviceDisconnected,
  };

  const handler = handlers[change_type];
  if (handler) {
    handler(data);
  }

  await refreshDevices();
  getBluetoothIcon();
};

const refreshDevices = async () => {
  if (!defaultAdapter.value) return;
  loading.value = true;
  try {
    connectedDevices.value = await getConnectedDevices(
      defaultAdapter.value.path
    );
    availableDevices.value = await getAvailableDevices(
      defaultAdapter.value.path
    );
    connectedDevicesCount.value = connectedDevices.value.length;
  } catch (e) {
    console.error("Error refreshing devices:", e);
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
    console.error("Error scanning for devices:", e);
  }
  isScanning.value = false;
};

// Lifecycle hooks
onMounted(async () => {
  defaultAdapter.value = await getDefaultAdapter();
  syncIcon.value = await getIconSource("emblem-synchronizing");
  await refreshDevices();
  await getBluetoothIcon();
  unlistenBluetooth.value = await listen(
    "bluetooth-change",
    handleBluetoothChange
  );
});

onUnmounted(() => {
  if (unlistenBluetooth.value) unlistenBluetooth.value();
});

const getBluetoothIcon = async () => {
  try {
    connectedDevicesCount.value = connectedDevices.value.length;
    let iconName = "bluetooth-disabled-symbolic";
    if (isBluetoothOn.value) {
      iconName =
        connectedDevicesCount.value > 0
          ? "bluetooth-active-symbolic"
          : "bluetooth-symbolic";
    }
    bluetoothIcon.value = await getIconSource(iconName);
  } catch (error) {
    console.error("Error loading bluetooth icon:", error);
    bluetoothIcon.value = "";
  }
};

const connect = async (device: any) => {
  await connectDevice(device.path);
  await refreshDevices();
};
const disconnect = async (device: any) => {
  await disconnectDevice(device.path);
  await refreshDevices();
};
</script>

<template>
  <div class="flex flex-col h-full">
    <div class="flex items-center mb-4">
      <button
        type="button"
        class="relative inline-flex items-center h-7 w-12 rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-vsk-primary mr-2"
        :class="isBluetoothOn ? 'bg-vsk-primary' : 'bg-gray-400'"
        @click="toggleBT"
        :disabled="isTogglingBluetooth"
      >
        <span
          class="inline-block h-6 w-6 transform rounded-full bg-white shadow transition-transform"
          :class="isBluetoothOn ? 'translate-x-5' : 'translate-x-1'"
        ></span>
      </button>
      <img :src="bluetoothIcon" alt="Bluetooth" class="h-8 w-auto mr-3" />
      <span class="font-bold text-2xl flex-1">Bluetooth</span>
      <button
        class="bg-vsk-primary text-white rounded-vsk px-1 py-0.5 active:bg-vsk-primary/80 disabled:cursor-not-allowed disabled:opacity-50"
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
            class="text-gray-500 text-sm px-1.5 text-center"
          >
            No hay dispositivos disponibles
          </div>
          <ul v-else class="list-none p-0 m-0">
            <li v-for="dev in availableDevices" :key="dev.path">
              <BluetoothDeviceCard
                :device="dev"
                action-label="Conectar"
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
            class="text-gray-500 text-sm px-1.5 text-center"
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
