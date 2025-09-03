<script lang="ts" setup>
import { ref, computed, onMounted, onUnmounted, Ref } from "vue";
import { getSymbolSource } from "@vasakgroup/plugin-vicons";
import {
  getDefaultAdapter,
  AdapterInfo,
  getConnectedDevicesCount,
} from "@vasakgroup/plugin-bluetooth-manager";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const connectedDevices: Ref<any[]> = ref([]);
const availableDevices: Ref<any[]> = ref([]);
const bluetoothIcon: Ref<string> = ref("");
const defaultAdapter = ref<AdapterInfo | null>(null);
const connectedDevicesCount: Ref<number> = ref(0);
let unlistenBluetooth: Ref<Function | null> = ref(null);

const isBluetoothOn = computed(() => {
  return defaultAdapter.value?.powered;
});

const handleBluetoothChange = (event: any) => {
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
      const deviceIndex = availableDevices.value.findIndex(
        (d) => d.path === data.path
      );
      if (deviceIndex !== -1) {
        if (data.connected) {
          availableDevices.value.splice(deviceIndex, 1);
          if (!connectedDevices.value.find((d) => d.path === data.path)) {
            connectedDevices.value.push(data);
          }
        } else {
          availableDevices.value[deviceIndex] = data;
        }
      } else {
        const connectedIndex = connectedDevices.value.findIndex(
          (d) => d.path === data.path
        );
        if (connectedIndex !== -1) {
          if (data.connected) {
            connectedDevices.value[connectedIndex] = data;
          } else {
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

onMounted(async () => {
  defaultAdapter.value = await getDefaultAdapter();
  await getBluetoothIcon();
  connectedDevicesCount.value = await getConnectedDevicesCount(
    defaultAdapter.value?.path as string
  );
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

const toggleBluetooth = async () => {
  try {
    await invoke("toggle_bluetooth_applet");
  } catch (error) {
    console.error("Error toggling bluetooth applet:", error);
  }
};

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

    bluetoothIcon.value = await getSymbolSource(iconName);
  } catch (error) {
    console.error("Error loading bluetooth icon:", error);
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
