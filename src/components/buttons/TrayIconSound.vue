<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { TrayIconButton } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, type Ref, ref, watch } from 'vue';
import type { UnlistenFn } from '@/interfaces/event';
import type { VolumeInfo } from '@/interfaces/volume';
import { logError } from '@/utils/logger';
import { calculateVolumePercentage, getVolumeIconName } from '@/utils/volume';

const volumeInfo: Ref<VolumeInfo> = ref({
	current: 0,
	min: 0,
	max: 100,
	is_muted: false,
});
const currentVolume: Ref<number> = ref(0);
const currentIcon: Ref<string> = ref('');
const unlistenVolume: Ref<UnlistenFn | null> = ref(null);

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
		const info = (await invoke('get_audio_volume')) as VolumeInfo;
		volumInfo.value = info;
		currentVolume.value = info.current;
		await updateIcon();
	} catch (error) {
		logError('Error getting volume:', error);
	}
}

async function toggleApplet(): Promise<void> {
	try {
		await invoke('toggle_audio_applet');
	} catch (error) {
		logError('Error toggling audio applet:', error);
	}
}

onMounted(async () => {
	unlistenVolume.value = await listen('volume-changed', (event) => {
		volumeInfo.value = event.payload as VolumeInfo;
		currentVolume.value = (event.payload as VolumeInfo).current;
		updateIcon();
	});
	await getVolumeInfo();
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