<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
/**
 * El volumen de salida. Con `devices`, lleva a la derecha la flecha › que abre
 * la elección de la salida dentro del bloque de ajustes del centro de control
 * (vasak-desktop#182).
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton, SliderControl } from '@vasakgroup/vue-libvasak';
import { onMounted } from 'vue';
import { useVolumeState } from '@/tools/composables/useVolumeState';
import { useSharedEvent } from '@/tools/event.bus';

withDefaults(defineProps<{ devices?: boolean }>(), { devices: false });
const emit = defineEmits<{ 'open-devices': [] }>();

const { t } = useI18n();

const {
	volumeInfo,
	currentVolume,
	currentIcon,
	getVolumeInfo,
	updateVolume,
	toggleMute,
	getPercentageClass,
} = useVolumeState();

onMounted(async () => {
	await getVolumeInfo();
});

// Otra salida por omisión trae otro volumen: se lee el de la nueva.
useSharedEvent('audio-devices-changed', () => {
	void getVolumeInfo();
});
</script>

<template>
  <div class="flex w-full min-w-0 items-center gap-1" data-volume-control>
    <SliderControl
      class="min-w-0 flex-1"
      :name="currentIcon"
      type="symbol"
      :label="t('components.VolumeControl.volume')"
      :button-label="volumeInfo.is_muted
        ? t('components.VolumeControl.unmute')
        : t('components.VolumeControl.mute')"
      v-model="currentVolume"
      :min="volumeInfo.min"
      :max="volumeInfo.max"
      :show-button="true"
      :icon-class="{ 'opacity-60': volumeInfo.is_muted }"
      :get-percentage-class="getPercentageClass"
      @update:model-value="updateVolume"
      @button-click="toggleMute"
    />
    <ActionButton
      v-if="devices"
      label=""
      icon="go-next"
      icon-type="symbol"
      :icon-alt="t('components.VolumeControl.devices')"
      :title="t('components.VolumeControl.devices')"
      variant="ghost"
      size="sm"
      class="shrink-0"
      data-sheet-opener="audio-output"
      @click="emit('open-devices')"
    />
  </div>
</template>
