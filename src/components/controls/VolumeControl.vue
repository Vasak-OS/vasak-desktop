<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, ref, watch } from 'vue';
import { SliderControl } from '@vasakgroup/vue-libvasak';
import type { UnlistenFn } from '@/interfaces/event';
import type { VolumeInfo } from '@/interfaces/volume';
import { logError } from '@/utils/logger';
import { calculateVolumePercentage, getVolumeIconName } from '@/utils/volume';

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
		const info = await invoke<VolumeInfo>('get_audio_volume');
		volumeInfo.value = info;
		currentVolume.value = info.current;
		await updateIcon();
	} catch (error) {
		logError('Error getting volume:', error);
	}
}

async function updateVolume(): Promise<void> {
	try {
		await invoke('set_audio_volume', { volume: currentVolume.value });
		await updateIcon();
	} catch (error) {
		logError('Error setting volume:', error);
	}
}

async function toggleMute(): Promise<void> {
	try {
		await invoke('toggle_audio_mute');
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

<style scoped>
:root {
	--slider-height: 6px;
	--thumb-size: 20px;
	--thumb-color: #00c951;
	--thumb-shadow: 0 2px 8px rgba(0, 0, 0, 0.15), 0 0 0 1px rgba(59, 130, 246, 0.2);
	--thumb-hover-scale: 1.25;
	--thumb-active-scale: 1.35;
	--thumb-hover-shadow: 0 4px 16px rgba(59, 130, 246, 0.4), 0 0 0 4px rgba(59, 130, 246, 0.1);
	--thumb-active-shadow: 0 2px 12px rgba(59, 130, 246, 0.6), 0 0 0 6px rgba(59, 130, 246, 0.2);
}

input[type='range'] {
	-webkit-appearance: none;
	appearance: none;
	background: rgba(0, 201, 81, 0.2);
	height: var(--slider-height);
	cursor: pointer;
	border-radius: 9999px;
}

input[type='range']::-webkit-slider-track {
	background: #e5e7eb;
	height: var(--slider-height);
	border-radius: 9999px;
	transition: all 0.2s ease;
}

input[type='range']::-webkit-slider-thumb {
	-webkit-appearance: none;
	appearance: none;
	height: var(--thumb-size);
	width: var(--thumb-size);
	border-radius: 50%;
	background: var(--thumb-color);
	cursor: pointer;
	box-shadow: var(--thumb-shadow);
	transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

input[type='range']:hover::-webkit-slider-thumb {
	transform: scale(var(--thumb-hover-scale));
	box-shadow: var(--thumb-hover-shadow);
}

input[type='range']:active::-webkit-slider-thumb {
	transform: scale(var(--thumb-active-scale));
	box-shadow: var(--thumb-active-shadow);
}

.dark input[type='range']::-webkit-slider-track {
	background: #4b5563;
}

.dark input[type='range']::-webkit-slider-thumb {
	background: #60a5fa;
	border-color: #1f2937;
}
</style>
