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
  scanForDevices
} from "@vasakgroup/plugin-bluetooth-manager";
import { listen } from "@tauri-apps/api/event";

const connectedDevices: Ref<any[]> = ref([]);
const availableDevices: Ref<any[]> = ref([]);
const isTogglingBluetooth: Ref<boolean> = ref(false);
const bluetoothIcon: Ref<string> = ref("");
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
  console.info(`${change_type}: ${JSON.stringify(data)}`);

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
    connectedDevices.value = await getConnectedDevices(defaultAdapter.value.path);
    availableDevices.value = await getAvailableDevices(defaultAdapter.value.path);
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
    // Manejo de error opcional
  }
  isScanning.value = false;
};

// Lifecycle hooks
onMounted(async () => {
  defaultAdapter.value = await getDefaultAdapter();
  await refreshDevices();
  await getBluetoothIcon();
  unlistenBluetooth.value = await listen("bluetooth-change", handleBluetoothChange);
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
  <div>
    <div class="flex mb-1 items-center mb-4">
      <img :src="bluetoothIcon" alt="Bluetooth" class="h-8 w-auto mr-3" />
      <span class="font-bold text-2xl flex-1">Bluetooth</span>
      <button
        class="bg-vsk-primary text-white rounded-vsk px-4 py-2 active:bg-vsk-primary/80 disabled:cursor-not-allowed disabled:opacity-50 mr-2"
        :class="{ on: isBluetoothOn }"
        @click="toggleBT"
        :disabled="isTogglingBluetooth"
      >
        {{ isBluetoothOn ? 'Desactivar' : 'Activar' }}
      </button>
      <button
        class="bg-vsk-primary text-white rounded-vsk px-4 py-2 active:bg-vsk-primary/80 disabled:cursor-not-allowed disabled:opacity-50"
        @click="scanDevices"
        :disabled="!isBluetoothOn || isScanning"
      >
        <span v-if="isScanning">Escaneando...</span>
        <span v-else>Escanear</span>
      </button>
    </div>
    <div v-if="loading" class="text-center px-6">Cargando...</div>
    <div v-else>
      <div class="mb-8 h-[240px] overflow-y-auto">
        <div class="mb-4 font-semibold text-xl">Disponibles</div>
        <div v-if="availableDevices.length === 0" class="text-gray-500 text-sm px-1.5 text-center">No hay dispositivos disponibles</div>
        <ul v-else class="list-none p-0 m-0">
          <li v-for="dev in availableDevices" :key="dev.path" class="flex items-center justify-between background rounded-vsk px-6 py-3 mb-4">
            <span>{{ dev.alias || dev.name || dev.address }}</span>
            <button class="bg-vsk-primary rounded-vsk px-4 py-2 text-sm font-semibold cursor-pointer hover:opacity-70" @click="connect(dev)">Conectar</button>
          </li>
        </ul>
      </div>
      
      <div class="mb-8 h-[240px] overflow-y-auto">
        <div class="mb-4 font-semibold text-xl">Dispositivos conectados</div>
        <div v-if="connectedDevices.length === 0" class="text-gray-500 text-sm px-1.5 text-center">Ningún dispositivo conectado</div>
        <ul v-else class="list-none p-0 m-0">
          <li v-for="dev in connectedDevices" :key="dev.path" class="flex items-center justify-between background rounded-vsk px-6 py-3 mb-4 border-l-4 border-green-500">
            <span>{{ dev.alias || dev.name || dev.address }}</span>
            <button class="bg-vsk-primary rounded-vsk px-4 py-2 text-sm font-semibold cursor-pointer hover:opacity-70" @click="disconnect(dev)">Desconectar</button>
          </li>
        </ul>
      </div>
    </div>
  </div>
</template>

<style scoped>
.empty {
  color: #aaa;
  font-size: 0.98rem;
  padding: 0.3rem 0;
}
.loading {
  text-align: center;
  color: #aaa;
  padding: 1.5rem 0;
}
</style>