<script setup lang="ts">
import { ref, computed, onMounted, watch, Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { listen } from '@tauri-apps/api/event';
import type { VolumeInfo } from '@/interfaces/volume';
import type { UnlistenFn } from '@/interfaces/event';
import { getVolumeIconName, calculateVolumePercentage } from '@/utils/volume';
import TrayIconButton from '@/components/base/TrayIconButton.vue';

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
		console.error('Error loading icon:', error);
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
		volumeInfo.value = info;
		currentVolume.value = info.current;
		await updateIcon();
	} catch (error) {
		console.error('Error getting volume:', error);
	}
}

async function toggleApplet(): Promise<void> {
	try {
		await invoke('toggle_audio_applet');
	} catch (error) {
		console.error('Error toggling mute:', error);
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