<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	LoadingState,
	OptionGroup,
	type OptionGroupOption,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, type Ref, ref } from 'vue';
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

/** Cada salida como opción del grupo: el nombre, su volumen y si es la predeterminada. */
const options = computed<OptionGroupOption<string>[]>(() =>
	devices.value.map((device) => ({
		value: device.id,
		label: getDeviceName(device),
		description: t('components.AudioDeviceSelector.volume').replace(
			'{0}',
			String(Math.round(device.volume * 100))
		),
		badge: device.is_default ? t('components.AudioDeviceSelector.default') : undefined,
	}))
);
</script>

<template>
  <div class="space-y-2">
    <!-- `tx-muted` y no `ui-surface`: la superficie es un fondo, y como color
         de texto sobre otra superficie no llegaba ni a 2:1. -->
    <div class="flex items-center gap-2 text-label-m font-medium text-tx-muted">
      <ThemeIcon name="audio-speakers-symbolic" type="symbol" :size="16" :alt="t('components.AudioDeviceSelector.speakerAlt')" />
      <span>{{ t('components.AudioDeviceSelector.title') }}</span>
    </div>

    <!-- Elegir una salida es elegir una de varias: `OptionGroup` de la
         librería (vue-libvasak 2.2.0), un `radiogroup` con un `radio` por fila,
         el punto con su contorno de 3:1 y la insignia «Predeterminado». Era
         una de las cuatro copias del mismo selector en el taller. -->
    <OptionGroup
      v-if="!isLoading && devices.length > 0"
      :model-value="selectedDeviceId"
      :options="options"
      :label="t('components.AudioDeviceSelector.title')"
      size="sm"
      @change="onDeviceChange"
    />

    <LoadingState v-else-if="isLoading" size="sm" :label="t('components.AudioDeviceSelector.loadingDevices')" />

    <p v-else class="text-label-xs text-tx-muted">
      {{ t('components.AudioDeviceSelector.noDevices') }}
    </p>
  </div>
</template>
