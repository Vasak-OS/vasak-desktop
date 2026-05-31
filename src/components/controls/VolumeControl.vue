<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { onMounted } from 'vue';
import { useVolumeState } from '@/tools/composables/useVolumeState';
import SliderControl from '../forms/SliderControl.vue';

const {
	volumeInfo,
	currentVolume,
	currentIcon,
	volumePercentage,
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
    :alt="volumeInfo.is_muted ? 'Unmute' : 'Mute'"
    :tooltip="volumeInfo.is_muted ? 'Unmute' : 'Mute'"
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

