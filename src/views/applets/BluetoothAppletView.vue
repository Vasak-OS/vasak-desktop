<script lang="ts" setup>
import { ref, computed, onMounted, onUnmounted, Ref } from "vue";
import { getIconSource } from "@vasakgroup/plugin-vicons";
import {
  getDefaultAdapter,
  toggleBluetooth,
  AdapterInfo,
  getConnectedDevicesCount,
} from "@vasakgroup/plugin-bluetooth-manager";
import { listen } from "@tauri-apps/api/event";

const connectedDevices: Ref<any[]> = ref([]);
const availableDevices: Ref<any[]> = ref([]);
const isTogglingBluetooth: Ref<boolean> = ref(false);
const bluetoothIcon: Ref<string> = ref("");
const defaultAdapter = ref<AdapterInfo | null>(null);
const connectedDevicesCount: Ref<number> = ref(0);

let unlistenBluetooth: Ref<Function | null> = ref(null);

const toggleBT = async () => {
  isTogglingBluetooth.value = true;
  try {
    await toggleBluetooth();
  } catch (error) {
    console.error("Error toggling Bluetooth:", error);
  } finally {
    isTogglingBluetooth.value = false;
  }
};

// Computed properties
const isBluetoothOn = computed(() => {
  return defaultAdapter.value?.powered;
});

// Manejador de eventos de Bluetooth
const handleBluetoothChange = (event: any) => {
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
  getBluetoothIcon();
};

// Lifecycle hooks
onMounted(async () => {
  defaultAdapter.value = await getDefaultAdapter();
  await getBluetoothIcon();
  connectedDevicesCount.value = await getConnectedDevicesCount(
    defaultAdapter.value?.path as string
  );
  console.info("Default Bluetooth Adapter:", connectedDevicesCount.value);
  unlistenBluetooth.value = await listen(
    "bluetooth-change",
    handleBluetoothChange
  );
});

onUnmounted(() => {
  if (unlistenBluetooth.value) {
    unlistenBluetooth.value();
  }
});

const getBluetoothIcon = async () => {
  try {
    connectedDevicesCount.value = await getConnectedDevicesCount(
      defaultAdapter.value?.path as string
    );
    const iconName = isBluetoothOn
      ? connectedDevicesCount.value > 0
        ? "bluetooth-active-symbolic"
        : "bluetooth-symbolic"
      : "bluetooth-disabled-symbolic";

    bluetoothIcon.value = await getIconSource(iconName);
  } catch (error) {
    console.error("Error loading bluetooth icon:", error);
  }
};
</script>

<template></template>
