<template>
  <div
    class="background rounded-vsk flex flex-row items-center gap-2 justify-between w-full h-auto p-4 transition-all duration-200 hover:bg-white/60 dark:hover:bg-black/60"
  >
    <button
      @click="toggleMute"
      class="w-8 h-8 flex items-center justify-center rounded-vsk transition-all duration-200 hover:bg-white/30 dark:hover:bg-black/30 hover:scale-110 active:scale-95"
    >
      <img
        :src="currentIcon"
        :alt="volumeInfo.is_muted ? 'Unmute' : 'Mute'"
        :title="volumeInfo.is_muted ? 'Unmute' : 'Mute'"
        class="w-6 h-6 transition-all duration-200"
        :class="{ 'opacity-60': volumeInfo.is_muted }"
      />
    </button>
    <input
      type="range"
      :min="volumeInfo.min"
      :max="volumeInfo.max"
      v-model="currentVolume"
      @input="updateVolume"
      class="flex-1 transition-all duration-200 hover:scale-105"
    />
    <span
      class="w-12 text-right transition-all duration-200 font-medium"
      :class="{
        'text-red-500': volumeInfo.is_muted,
        'text-green-500': volumePercentage > 80,
      }"
      >{{ volumePercentage }}%</span
    >
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { listen } from '@tauri-apps/api/event';

const volumeInfo: Ref<any> = ref({
	current: 0,
	min: 0,
	max: 100,
	is_muted: false,
});
const currentVolume: Ref<number> = ref(0);
const currentIcon: Ref<string> = ref('');
const unlistenVolume: Ref<Function | null> = ref(null);

async function updateIcon() {
	const getIconName = () => {
		if (volumeInfo.value.is_muted) return 'audio-volume-muted-symbolic';

		const percentage = volumePercentage.value;
		if (percentage <= 0) return 'audio-volume-muted-symbolic';
		if (percentage <= 33) return 'audio-volume-low-symbolic';
		if (percentage <= 66) return 'audio-volume-medium-symbolic';
		return 'audio-volume-high-symbolic';
	};

	try {
		currentIcon.value = await getSymbolSource(getIconName());
	} catch (error) {
		console.error('Error loading icon:', error);
	}
}

const volumePercentage = computed(() => {
	const range = volumeInfo.value.max - volumeInfo.value.min;
	const current = currentVolume.value - volumeInfo.value.min;
	return Math.round((current / range) * 100);
});

// Observar cambios en el volumen y estado de mute
watch([() => volumeInfo.value.is_muted, volumePercentage], updateIcon, {
	immediate: true,
});

async function getVolumeInfo() {
	try {
		const info: any = await invoke('get_audio_volume');
		volumeInfo.value = info;
		currentVolume.value = info.current;
		await updateIcon();
	} catch (error) {
		console.error('Error getting volume:', error);
	}
}

async function updateVolume() {
	try {
		await invoke('set_audio_volume', {
			volume: Number(currentVolume.value),
		});
	} catch (error) {
		console.error('Error setting volume:', error);
	}
}

async function toggleMute() {
	try {
		const isUnmuted = await invoke('toggle_audio_mute');
		volumeInfo.value.is_muted = !isUnmuted;
		await getVolumeInfo();
		await updateIcon();
	} catch (error) {
		console.error('Error toggling mute:', error);
	}
}

onMounted(async () => {
	unlistenVolume.value = await listen('volume-changed', (event) => {
		volumeInfo.value = event.payload;
		currentVolume.value = (event.payload as any).current;
		updateIcon();
	});
	await getVolumeInfo();
});
</script>

<style scoped>
@reference '../../style.css';

input[type="range"] {
  -webkit-appearance: none;
  appearance: none;
  background: rgba(0, 201, 81, 0.2);
  height: 6px;
  cursor: pointer;
  border-radius: 9999px;
}

input[type="range"]::-webkit-slider-track {
  background: #e5e7eb;
  height: 6px;
  border-radius: 9999px;
  transition: all 0.2s ease;
}

input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  height: 20px;
  width: 20px;
  border-radius: 50%;
  background: #00c951;
  cursor: pointer;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15), 0 0 0 1px rgba(59, 130, 246, 0.2);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

input[type="range"]:hover::-webkit-slider-thumb {
  transform: scale(1.25);
  box-shadow: 0 4px 16px rgba(59, 130, 246, 0.4),
    0 0 0 4px rgba(59, 130, 246, 0.1);
  background: #00c951;
}

input[type="range"]:active::-webkit-slider-thumb {
  transform: scale(1.35);
  box-shadow: 0 2px 12px rgba(59, 130, 246, 0.6),
    0 0 0 6px rgba(59, 130, 246, 0.2);
  background: #00c951;
}

.dark input[type="range"]::-webkit-slider-track {
  background: #4b5563;
}

.dark input[type="range"]::-webkit-slider-thumb {
  background: #60a5fa;
  border-color: #1f2937;
}
</style>
