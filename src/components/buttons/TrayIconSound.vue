<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, ref, watch } from 'vue';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import type { VolumeInfo } from '@/interfaces/volume';
import { getAudioVolume } from '@/services/core.service';
import { toggleAudioApplet } from '@/services/window.service';
import { useEventListener } from '@/tools/event.listener';
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

watch([() => volumeInfo.value.is_muted, volumePercentage], updateIcon, {
	immediate: true,
});

async function getVolumeInfo(): Promise<void> {
	try {
		const info = (await getAudioVolume()) as VolumeInfo;
		volumeInfo.value = info;
		currentVolume.value = info.current;
		await updateIcon();
	} catch (error) {
		logError('Error getting volume:', error);
	}
}

async function toggleApplet(): Promise<void> {
	try {
		await toggleAudioApplet();
	} catch (error) {
		logError('Error toggling audio applet:', error);
	}
}

onMounted(async () => {
	await getVolumeInfo();
});

useEventListener<VolumeInfo>('volume-changed', (event) => {
	volumeInfo.value = event.payload;
	currentVolume.value = event.payload.current;
	updateIcon();
});
</script>
<template>
  <TrayIconButton
    :icon="currentIcon"
    :tooltip="volumeInfo.is_muted ? 'Unmute' : 'Mute'"
    :alt="volumeInfo.is_muted ? 'Unmute' : 'Mute'"
    :icon-class="{ 'opacity-60': volumeInfo.is_muted }"
    @click="toggleApplet"
  />
</template>