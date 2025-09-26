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

const handleBluetoothChange = async (event: any) => {
  const { change_type, data } = event.payload;

  switch (change_type) {
    case "adapter-property-changed":
      if (defaultAdapter.value && data.path === defaultAdapter.value.path) {
        defaultAdapter.value = data;
      }
      break;

    case "device-added":
      if (!availableDevices.value.find((d) => d.path === data.path)) {
        availableDevices.value.push(data);
      }
      break;

    case "device-removed":
      availableDevices.value = availableDevices.value.filter(
        (d) => d.path !== data.path
      );
      connectedDevices.value = connectedDevices.value.filter(
        (d) => d.path !== data.path
      );
      break;

    case "device-connected":
    case "device-property-changed":
      // Actualizar en ambas listas
      const deviceIndex = availableDevices.value.findIndex(
        (d) => d.path === data.path
      );
      if (deviceIndex !== -1) {
        if (data.connected) {
          // Mover a conectados
          availableDevices.value.splice(deviceIndex, 1);
          if (!connectedDevices.value.find((d) => d.path === data.path)) {
            connectedDevices.value.push(data);
          }
        } else {
          // Actualizar en disponibles
          availableDevices.value[deviceIndex] = data;
        }
      } else {
        // Actualizar en conectados
        const connectedIndex = connectedDevices.value.findIndex(
          (d) => d.path === data.path
        );
        if (connectedIndex !== -1) {
          if (data.connected) {
            connectedDevices.value[connectedIndex] = data;
          } else {
            // Mover a disponibles
            connectedDevices.value.splice(connectedIndex, 1);
            if (!availableDevices.value.find((d) => d.path === data.path)) {
              availableDevices.value.push(data);
            }
          }
        }
      }
      break;

    case "device-disconnected":
      const disconnectedIndex = connectedDevices.value.findIndex(
        (d) => d.path === data.path
      );
      if (disconnectedIndex !== -1) {
        connectedDevices.value.splice(disconnectedIndex, 1);
        if (!availableDevices.value.find((d) => d.path === data.path)) {
          availableDevices.value.push(data);
        }
      }
      break;
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
    const iconName = isBluetoothOn.value
      ? connectedDevicesCount.value > 0
        ? "bluetooth-active-symbolic"
        : "bluetooth-symbolic"
      : "bluetooth-disabled-symbolic";
    bluetoothIcon.value = await getIconSource(iconName);
  } catch (error) {
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

<style scoped></style>
