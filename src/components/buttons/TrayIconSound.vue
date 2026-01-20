<template>
  <div
    class="p-1 rounded-vsk relative hover:bg-vsk-primary/30"
    @click="toggleApplet"
  >
    <img
      :src="currentIcon"
      :alt="volumeInfo.is_muted ? 'Unmute' : 'Mute'"
      :title="volumeInfo.is_muted ? 'Unmute' : 'Mute'"
      class="m-auto h-[22px] w-auto transition-all duration-3000"
      :class="{ 'opacity-60': volumeInfo.is_muted }"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { listen } from '@tauri-apps/api/event';
import type { VolumeInfo } from '@/interfaces/volume';
import type { UnlistenFn } from '@/interfaces/event';

const volumeInfo: Ref<VolumeInfo> = ref({
	current: 0,
	min: 0,
	max: 100,
	is_muted: false,
});
const currentVolume: Ref<number> = ref(0);
const currentIcon: Ref<string> = ref('');
const unlistenVolume: Ref<UnlistenFn | null> = ref(null);

function getIconName(isMuted: boolean, percentage: number): string {
	if (isMuted) return 'audio-volume-muted-symbolic';
	if (percentage <= 0) return 'audio-volume-muted-symbolic';
	if (percentage <= 33) return 'audio-volume-low-symbolic';
	if (percentage <= 66) return 'audio-volume-medium-symbolic';
	return 'audio-volume-high-symbolic';
}

async function updateIcon(): Promise<void> {
	try {
		const iconName = getIconName(volumeInfo.value.is_muted, volumePercentage.value);
		currentIcon.value = await getSymbolSource(iconName);
	} catch (error) {
		console.error('Error loading icon:', error);
	}
}

const volumePercentage = computed(() => {
	const range = volumeInfo.value.max - volumeInfo.value.min;
	const current = currentVolume.value - volumeInfo.value.min;
	return Math.round((current / range) * 100);
});

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
