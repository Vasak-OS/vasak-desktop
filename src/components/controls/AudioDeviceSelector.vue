<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ThemeIcon } from '@vasakgroup/vue-libvasak';
import { onMounted, type Ref, ref } from 'vue';
import { getAudioDevices, setAudioDevice } from '@/services/core.service';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

const { t } = useI18n();

interface AudioDevice {
	id: string;
	name: string;
	description: string;
	is_default: boolean;
	volume: number;
}

const devices: Ref<AudioDevice[]> = ref([]);
const selectedDeviceId = ref('');
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
	await loadDevices();
});

useSharedEvent<AudioDevice[]>('audio-devices-changed', (payload) => {
	devices.value = payload;
	const defaultDevice = payload.find((d) => d.is_default);
	if (defaultDevice) {
		selectedDeviceId.value = defaultDevice.id;
	} else if (payload.length > 0) {
		selectedDeviceId.value = payload[0].id;
	} else {
		selectedDeviceId.value = '';
	}
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
    <!-- `tx-muted` y no `ui-surface`: la superficie es un fondo, y como color
         de texto sobre otra superficie no llegaba ni a 2:1. -->
    <div class="flex items-center gap-2 text-label-m font-medium text-tx-muted">
      <ThemeIcon name="audio-speakers-symbolic" type="symbol" :size="16" :alt="t('components.AudioDeviceSelector.speakerAlt')" />
      <span>{{ t('components.AudioDeviceSelector.title') }}</span>
    </div>

    <!--
      Elegir una salida de audio es elegir una de varias, no apretar botones
      sueltos: por eso el grupo es un `radiogroup` y cada fila un `radio`. Así
      se anuncia cuál está puesta —que es lo que el punto de la izquierda dibuja
      y un lector de pantalla no puede ver— en vez de leer cinco botones
      iguales.

      Acá sí va un `<button>` de verdad, y no `role="button"` sobre un `<div>`
      como en las tarjetas: esta fila no tiene ningún botón adentro, así que no
      hay nada que anidar.
    -->
    <div v-if="!isLoading && devices.length > 0" class="space-y-1" role="radiogroup"
      :aria-label="t('components.AudioDeviceSelector.title')">
      <button v-for="device in devices" :key="device.id" type="button" role="radio"
        :aria-checked="selectedDeviceId === device.id"
        class="flex w-full min-w-0 items-center gap-2 p-2 rounded-corner-m cursor-pointer text-left transition-colors duration-200 ease-ui focus-visible:-outline-offset-2" :class="[
          selectedDeviceId === device.id
            ? 'bg-ui-selected-accent font-semibold'
            : 'hover:bg-ui-hover active:bg-ui-pressed',
        ]" @click="onDeviceChange(device.id)">

        <!-- El punto de la opción: el contorno de un control lleva 3:1
             (`ui-border-strong`), y la elegida, el primario con el punto en el
             texto que va encima del primario. Nada de blanco escrito a mano. -->
        <div
          class="w-4 h-4 shrink-0 rounded-corner-full border-2 flex items-center justify-center transition-colors duration-200 ease-ui"
          :class="[
            selectedDeviceId === device.id
              ? 'bg-primary border-primary'
              : 'border-ui-border-strong',
          ]">
          <div v-if="selectedDeviceId === device.id" class="w-2 h-2 bg-tx-on-primary rounded-corner-full" />
        </div>

        <!-- Device info -->
        <div class="flex-1 min-w-0">
          <div class="text-label-xs font-medium break-words">
            {{ getDeviceName(device) }}
          </div>
          <div class="text-label-xs text-tx-muted">
            {{ t('components.AudioDeviceSelector.volume').replace('{0}', String(Math.round(device.volume * 100))) }}
          </div>
        </div>

        <!-- La insignia de la librería (la de `SideButton`): antes el texto iba
             en `primary` sobre fondo `primary`, o sea que no se leía. -->
        <div v-if="device.is_default"
          class="flex h-5 shrink-0 items-center rounded-corner-full bg-ui-selected px-2 text-label-xs font-semibold text-tx-main">
          {{ t('components.AudioDeviceSelector.default') }}
        </div>
      </button>
    </div>

    <div v-else-if="isLoading" class="text-label-xs text-tx-muted">
      {{ t('components.AudioDeviceSelector.loadingDevices') }}
    </div>

    <div v-else class="text-label-xs text-tx-muted">
      {{ t('components.AudioDeviceSelector.noDevices') }}
    </div>
  </div>
</template>
