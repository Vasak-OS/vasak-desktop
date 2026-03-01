import { getAudioDevices, setAudioDevice } from '@/services/core.service';

<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { onMounted, type Ref, ref } from 'vue';
import { getAudioDevices, setAudioDevice } from '@/services/audio.service';
import { logError } from '@/utils/logger';

interface AudioDevice {
	id: string;
	name: string;
	description: string;
	is_default: boolean;
	volume: number;
}

const devices: Ref<AudioDevice[]> = ref([]);
const selectedDeviceId = ref('');
const speakerIcon: Ref<string> = ref('');
const isLoading = ref(false);

async function loadDevices() {
	isLoading.value = true;
	try {
		const deviceList = await getAudioDevices();
		devices.value = deviceList;

		const defaultDevice = deviceList.find((d: any) => d.is_default);
		if (defaultDevice) {
			selectedDeviceId.value = defaultDevice.id;
		}
	} catch (e) {
		logError('[audio] Failed to load devices:', e);
	} finally {
		isLoading.value = false;
	}
}

async function onDeviceChange(deviceId: string) {
	selectedDeviceId.value = deviceId;
	try {
		await setAudioDevice({ deviceId });
		await loadDevices();
	} catch (e) {
		logError('[audio] Failed to set device:', e);
	}
}

onMounted(async () => {
	speakerIcon.value = await getSymbolSource('audio-speakers-symbolic');
	await loadDevices();

	listen<AudioDevice[]>('audio-devices-changed', (event) => {
		devices.value = event.payload;
		const defaultDevice = event.payload.find((d) => d.is_default);
		if (defaultDevice) {
			selectedDeviceId.value = defaultDevice.id;
		}
	});
});

function getDeviceName(device: AudioDevice): string {
	return device.name
		.replaceAll('ALSA', '')
		.replaceAll('PulseAudio', '')
		.replaceAll('PipeWire', '')
		.trim();
}
</script>

<template>
  <div class="space-y-2">
    <div class="flex items-center gap-2 text-sm font-medium text-muted">
      <img v-if="speakerIcon" :src="speakerIcon" alt="Speaker" class="w-4 h-4" />
      <span>Dispositivo de audio</span>
    </div>

    <div v-if="!isLoading && devices.length > 0" class="space-y-1">
      <div v-for="device in devices" :key="device.id"
        class="flex items-center gap-2 p-2 rounded-corner cursor-pointer transition-colors" :class="[
          selectedDeviceId === device.id
            ? 'bg-primary dark:bg-primary-dark ring-1 ring-secondary dark:ring-secondary-dark'
            : 'bg-background hover:bg-primary hover:dark:bg-primary-dark',
        ]" @click="onDeviceChange(device.id)">

        <div
          class="w-4 h-4 rounded-full border-2 border-primary dark:border-primary-dark flex items-center justify-center transition-colors"
          :class="[
            selectedDeviceId === device.id
              ? 'bg-primary border-primary'
              : '',
          ]">
          <div v-if="selectedDeviceId === device.id" class="w-2 h-2 bg-white rounded-full" />
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

        <div v-if="device.is_default"
          class="px-2 py-0.5 bg-primary dark:bg-primary-dark rounded text-xs font-medium text-primary">
          Default
        </div>
      </div>
    </div>

    <div v-else-if="isLoading" class="text-xs text-muted">
      Cargando dispositivos...
    </div>

    <div v-else class="text-xs text-tx-muted">
      No hay dispositivos disponibles
    </div>
  </div>
</template>
