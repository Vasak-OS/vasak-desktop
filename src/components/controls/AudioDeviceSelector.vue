<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
/**
 * Elegir la salida o la entrada de audio por omisión. `kind="input"` es la
 * ficha del micrófono del centro de control (vasak-desktop#182): las mismas
 * filas, con las entradas y su evento.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	LoadingState,
	OptionGroup,
	type OptionGroupOption,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, type Ref, ref, watch } from 'vue';
import { getAudioInputDevices, setAudioInputDevice } from '@/services/audio-input.service';
import { getAudioDevices, setAudioDevice } from '@/services/core.service';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

const props = withDefaults(defineProps<{ kind?: 'output' | 'input' }>(), { kind: 'output' });

const { t } = useI18n();

interface AudioDevice {
	id: string;
	name: string;
	description: string;
	is_default: boolean;
	volume: number;
}

/**
 * Lo que cambia entre salida y entrada: de dónde se leen, cómo se eligen y el
 * evento. Sigue a `kind` aunque cambie con el componente montado.
 */
const SOURCES = {
	input: {
		read: () => getAudioInputDevices(),
		choose: (deviceId: string) => setAudioInputDevice(deviceId),
		event: 'audio-input-devices-changed',
		icon: 'audio-input-microphone-symbolic',
		title: 'components.AudioDeviceSelector.inputTitle',
		iconAlt: 'components.AudioDeviceSelector.microphoneAlt',
	},
	output: {
		read: () => getAudioDevices<AudioDevice[]>(),
		choose: (deviceId: string) => setAudioDevice({ deviceId }),
		event: 'audio-devices-changed',
		icon: 'audio-speakers-symbolic',
		title: 'components.AudioDeviceSelector.title',
		iconAlt: 'components.AudioDeviceSelector.speakerAlt',
	},
} as const;
const source = computed(() => SOURCES[props.kind]);

const devices: Ref<AudioDevice[]> = ref([]);
const selectedDeviceId = ref('');
const isLoading = ref(false);

async function loadDevices() {
	isLoading.value = true;
	try {
		const deviceList = await source.value.read();
		devices.value = deviceList;

		const defaultDevice = deviceList.find((d) => d.is_default);
		if (defaultDevice) {
			selectedDeviceId.value = defaultDevice.id;
		}
	} catch (e) {
		logError('[audio] Failed to load devices:', e);
	} finally {
		isLoading.value = false;
	}
}

/**
 * La salida marcada en el grupo, **controlada**: sólo cambia cuando el sistema
 * confirma. `OptionGroup` mueve su marca antes de avisar, y sin quien escuche
 * `update:model-value` se queda con esa marca aunque el cambio falle —la
 * salida que no se pudo poner quedaba elegida—. Con el setter vacío manda el
 * valor de acá, que es el que leyó el sistema.
 */
const checkedDevice = computed<string | null>({
	get: () => selectedDeviceId.value || null,
	set: () => {},
});

async function onDeviceChange(deviceId: string) {
	try {
		await source.value.choose(deviceId);
		selectedDeviceId.value = deviceId;
		await loadDevices();
	} catch (e) {
		logError('[audio] Failed to set device:', e);
	}
}

onMounted(async () => {
	await loadDevices();
});

function applyDevices(payload: AudioDevice[]): void {
	devices.value = payload;
	const defaultDevice = payload.find((d) => d.is_default);
	if (defaultDevice) {
		selectedDeviceId.value = defaultDevice.id;
	} else if (payload.length > 0) {
		selectedDeviceId.value = payload[0].id;
	} else {
		selectedDeviceId.value = '';
	}
}

// Los dos eventos, y cada uno cuenta sólo si es el de la clase de ahora.
for (const kind of ['output', 'input'] as const) {
	useSharedEvent<AudioDevice[]>(SOURCES[kind].event, (payload) => {
		if (props.kind === kind) applyDevices(payload);
	});
}

// Otra clase con el componente montado: se leen sus dispositivos.
watch(
	() => props.kind,
	() => {
		devices.value = [];
		selectedDeviceId.value = '';
		void loadDevices();
	}
);

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
      <ThemeIcon :name="source.icon" type="symbol" :size="16" :alt="t(source.iconAlt)" />
      <span>{{ t(source.title) }}</span>
    </div>

    <!-- Elegir una salida es elegir una de varias: `OptionGroup` de la
         librería (vue-libvasak 2.2.0), un `radiogroup` con un `radio` por fila,
         el punto con su contorno de 3:1 y la insignia «Predeterminado». Era
         una de las cuatro copias del mismo selector en el taller. -->
    <OptionGroup
      v-if="!isLoading && devices.length > 0"
      v-model="checkedDevice"
      :options="options"
      :label="t(source.title)"
      size="sm"
      @change="onDeviceChange"
    />

    <LoadingState v-else-if="isLoading" size="sm" :label="t('components.AudioDeviceSelector.loadingDevices')" />

    <p v-else class="text-label-xs text-tx-muted">
      {{ t('components.AudioDeviceSelector.noDevices') }}
    </p>
  </div>
</template>
