<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, ref, watch } from 'vue';
import type { UnlistenFn } from '@/interfaces/event';
import type { VolumeInfo } from '@/interfaces/volume';
import { getAudioVolume, setAudioVolume } from '@/services/audio.service';
import { toggleAudioMute } from '@/services/core.service';
import { logError } from '@/utils/logger';
import { calculateVolumePercentage, getVolumeIconName } from '@/utils/volume';
import SliderControl from '../forms/SliderControl.vue';

const volumeInfo = ref<VolumeInfo>({
	current: 0,
	min: 0,
	max: 100,
	is_muted: false,
});
const currentVolume = ref(0);
const currentIcon = ref('');
const unlistenVolume = ref<UnlistenFn | null>(null);

async function updateIcon(): Promise<void> {
	try {
		const percentage = calculateVolumePercentage(volumeInfo.value, currentVolume.value);
		const iconName = getVolumeIconName(volumeInfo.value.is_muted, percentage);
		currentIcon.value = await getSymbolSource(iconName);
	} catch (error) {
		logError('Error loading volume icon:', error);
	}
}

const volumePercentage = computed(() =>
	calculateVolumePercentage(volumeInfo.value, currentVolume.value)
);

// Watch for changes in mute status and volume percentage
watch([() => volumeInfo.value.is_muted, volumePercentage], updateIcon, {
	immediate: true,
});

async function getVolumeInfo(): Promise<void> {
	try {
		const info = await getAudioVolume();
		volumeInfo.value = info;
		currentVolume.value = info.current;
		await updateIcon();
	} catch (error) {
		logError('Error getting volume:', error);
	}
}

async function updateVolume(): Promise<void> {
	try {
		await setAudioVolume({ volume: currentVolume.value });
		await updateIcon();
	} catch (error) {
		logError('Error setting volume:', error);
	}
}

async function toggleMute(): Promise<void> {
	try {
		await toggleAudioMute();
		await getVolumeInfo();
	} catch (error) {
		logError('Error toggling mute:', error);
	}
}

const getPercentageClass = (percentage: number) => {
	if (volumeInfo.value.is_muted) return 'text-red-500';
	if (percentage > 80) return 'text-green-500';
	return '';
};

onMounted(async () => {
	await getVolumeInfo();
	unlistenVolume.value = await listen<VolumeInfo>('volume-changed', async (event) => {
		volumeInfo.value = event.payload;
		currentVolume.value = event.payload.current;
		await updateIcon();
	});
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

