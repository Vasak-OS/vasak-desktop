<script lang="ts" setup>
import { onMounted, ref, Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getSymbolSource } from "@vasakgroup/plugin-vicons";

interface AudioDevice {
  id: string;
  name: string;
  description: string;
  is_default: boolean;
  volume: number;
}

const devices: Ref<AudioDevice[]> = ref([]);
const selectedDeviceId = ref("");
const speakerIcon: Ref<string> = ref("");
const isLoading = ref(false);

async function loadDevices() {
  isLoading.value = true;
  try {
    const deviceList = await invoke<AudioDevice[]>("get_audio_devices");
    devices.value = deviceList;
    
    // Establecer el dispositivo seleccionado
    const defaultDevice = deviceList.find(d => d.is_default);
    if (defaultDevice) {
      selectedDeviceId.value = defaultDevice.id;
    }
  } catch (e) {
    console.error("[audio] Failed to load devices:", e);
  } finally {
    isLoading.value = false;
  }
}

async function onDeviceChange(deviceId: string) {
  selectedDeviceId.value = deviceId;
  try {
    await invoke("set_audio_device", { deviceId });
    // Recargar para confirmar cambio
    await loadDevices();
  } catch (e) {
    console.error("[audio] Failed to set device:", e);
  }
}

onMounted(async () => {
  speakerIcon.value = await getSymbolSource("audio-speakers-symbolic");
  await loadDevices();
  
  // Escuchar cambios de dispositivos desde el backend
  listen<AudioDevice[]>("audio-devices-changed", (event) => {
    devices.value = event.payload;
    const defaultDevice = event.payload.find(d => d.is_default);
    if (defaultDevice) {
      selectedDeviceId.value = defaultDevice.id;
    }
  });
});

// Obtener nombre corto del dispositivo (sin números de ID)
function getDeviceName(device: AudioDevice): string {
  // Remover prefixes comunes y dejar nombre limpio
  return device.name
    .replace(/ALSA/g, "")
    .replace(/PulseAudio/g, "")
    .replace(/PipeWire/g, "")
    .replace(/\[.*\]/g, "")
    .trim();
}
</script>

<template>
  <div class="space-y-2">
    <!-- Header -->
    <div class="flex items-center gap-2 text-sm font-medium text-muted">
      <img v-if="speakerIcon" :src="speakerIcon" alt="Speaker" class="w-4 h-4" />
      <span>Dispositivo de audio</span>
    </div>

    <!-- Device selector -->
    <div v-if="!isLoading && devices.length > 0" class="space-y-1">
      <div
        v-for="device in devices"
        :key="device.id"
        class="flex items-center gap-2 p-2 rounded-vsk cursor-pointer transition-colors"
        :class="[
          selectedDeviceId === device.id
            ? 'bg-vsk-primary/30 ring-1 ring-vsk-primary'
            : 'bg-background hover:bg-vsk-primary/10'
        ]"
        @click="onDeviceChange(device.id)"
      >
        <!-- Radio button -->
        <div
          class="w-4 h-4 rounded-full border-2 border-vsk-primary/50 flex items-center justify-center transition-colors"
          :class="[
            selectedDeviceId === device.id ? 'bg-vsk-primary border-vsk-primary' : ''
          ]"
        >
          <div
            v-if="selectedDeviceId === device.id"
            class="w-2 h-2 bg-white rounded-full"
          />
        </div>

        <!-- Device info -->
        <div class="flex-1 min-w-0">
          <div class="text-xs font-medium truncate">
            {{ getDeviceName(device) }}
          </div>
          <div class="text-xs text-muted/70">
            Vol: {{ Math.round(device.volume * 100) }}%
          </div>
        </div>

        <!-- Badge para dispositivo por defecto -->
        <div
          v-if="device.is_default"
          class="px-2 py-0.5 bg-vsk-primary/20 rounded text-xs font-medium text-vsk-primary"
        >
          Default
        </div>
      </div>
    </div>

    <!-- Loading state -->
    <div v-else-if="isLoading" class="text-xs text-muted">
      Cargando dispositivos...
    </div>

    <!-- No devices state -->
    <div v-else class="text-xs text-muted/50">
      No hay dispositivos disponibles
    </div>
  </div>
</template>

<style scoped>
</style>
