<template>
  <button
    @click="toggleBluetooth"
    class="p-2 rounded-xl bg-white/50 dark:bg-black/50 hover:bg-white/70 dark:hover:bg-black/70 transition-all duration-300 h-[70px] w-[70px] group relative overflow-hidden hover:scale-105 hover:shadow-lg active:scale-95"
    :class="{
      'animate-pulse': isLoading,
      'ring-2 ring-blue-400/50': bluetoothState.enabled,
      'opacity-60': !bluetoothState.enabled,
    }"
    :disabled="isLoading"
  >
    <!-- Background glow effect -->
    <div
      class="absolute inset-0 rounded-xl bg-gradient-to-br from-blue-500/20 to-indigo-500/20 opacity-0 group-hover:opacity-100 transition-opacity duration-300"
    ></div>

    <!-- Status indicator -->
    <div
      class="absolute top-1 right-1 w-3 h-3 rounded-full transition-all duration-300"
      :class="{
        'bg-blue-400 animate-pulse':
          bluetoothState.enabled && bluetoothState.connected_devices.length > 0,
        'bg-blue-400':
          bluetoothState.enabled &&
          bluetoothState.connected_devices.length === 0,
        'bg-gray-400': !bluetoothState.enabled,
      }"
    ></div>

    <!-- Connected devices count -->
    <div
      v-if="
        bluetoothState.enabled && bluetoothState.connected_devices.length > 0
      "
      class="absolute bottom-1 right-1 bg-blue-500 text-white text-xs rounded-full w-4 h-4 flex items-center justify-center font-bold animate-bounce"
    >
      {{ bluetoothState.connected_devices.length }}
    </div>

    <!-- Bluetooth waves animation when enabled -->
    <div
      v-if="bluetoothState.enabled"
      class="absolute inset-0 flex items-center justify-center"
    >
      <div
        class="absolute w-8 h-8 border-2 border-blue-400/30 rounded-full animate-ping"
      ></div>
      <div
        class="absolute w-12 h-12 border-2 border-blue-400/20 rounded-full animate-ping"
        style="animation-delay: 0.5s"
      ></div>
    </div>

    <img
      :src="bluetoothIcon"
      :alt="bluetoothAlt"
      class="m-auto w-[50px] h-[50px] transition-all duration-300 group-hover:scale-110 relative z-10"
      :class="{
        'animate-spin': isLoading,
        'filter brightness-75': !bluetoothState.enabled,
        'drop-shadow-lg': bluetoothState.enabled,
      }"
    />
  </button>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getIconSource } from "@vasakgroup/plugin-vicons";
import { listen } from "@tauri-apps/api/event";

interface BluetoothState {
  enabled: boolean;
  connected_devices: string[];
}

const bluetoothState = ref<BluetoothState>({
  enabled: false,
  connected_devices: [],
});

const bluetoothIcon: Ref<string> = ref("");
const isLoading: Ref<boolean> = ref(false);
const deviceConnectionListener: Ref<Function | null> = ref(null);

const getBluetoothIcon = async () => {
  try {
    const iconName = bluetoothState.value.enabled
      ? bluetoothState.value.connected_devices.length > 0
        ? "bluetooth-active-symbolic"
        : "bluetooth-symbolic"
      : "bluetooth-disabled-symbolic";

    bluetoothIcon.value = await getIconSource(iconName);
  } catch (error) {
    console.error("Error loading bluetooth icon:", error);
  }
};

const bluetoothAlt = computed(() => {
  if (!bluetoothState.value.enabled) return "Bluetooth deshabilitado";
  return bluetoothState.value.connected_devices.length > 0
    ? `${bluetoothState.value.connected_devices.length} dispositivos conectados`
    : "Bluetooth activo";
});

const updateBluetoothState = async () => {
  try {
    bluetoothState.value = await invoke("get_bluetooth_state");
    await getBluetoothIcon();
  } catch (error) {
    console.error("Error getting bluetooth state:", error);
  }
};

const toggleBluetooth = async () => {
  if (isLoading.value) return;

  isLoading.value = true;
  try {
    await invoke("toggle_bluetooth", {
      enable: !bluetoothState.value.enabled,
    });
    await updateBluetoothState();
  } catch (error) {
    console.error("Error toggling bluetooth:", error);
  } finally {
    isLoading.value = false;
  }
};

let interval: number;

onMounted(async () => {
  await updateBluetoothState();
  interval = window.setInterval(updateBluetoothState, 5000);
  deviceConnectionListener.value = await listen(
    "bluetooth-change",
    async (event) => {
      console.log("Estado de Bluetooth cambiado:", event.payload);
    }
  );
});

onUnmounted(() => {
  if (interval) {
    clearInterval(interval);
  }
  if (deviceConnectionListener.value) {
    deviceConnectionListener.value();
  }
});
</script>
