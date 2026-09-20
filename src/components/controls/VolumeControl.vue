<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { SliderControl } from '@vasakgroup/vue-libvasak';
import { onMounted } from 'vue';
import { useVolumeState } from '@/tools/composables/useVolumeState';

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
</script>

<template>
  <SliderControl
    :icon="currentIcon"
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
</template>

