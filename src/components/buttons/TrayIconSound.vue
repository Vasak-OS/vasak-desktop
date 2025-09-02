<template>
  <div
    class="p-1 rounded-vsk relative hover:bg-vsk-primary"
    @click="toggleMute"
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
import { ref, computed, onMounted, watch, Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getSymbolSource } from "@vasakgroup/plugin-vicons";

const volumeInfo: Ref<any> = ref({
  current: 0,
  min: 0,
  max: 100,
  is_muted: false,
});
const currentVolume: Ref<number> = ref(0);
const currentIcon: Ref<string> = ref("");

async function updateIcon() {
  const getIconName = () => {
    if (volumeInfo.value.is_muted) return "audio-volume-muted-symbolic";

    const percentage = volumePercentage.value;
    if (percentage <= 0) return "audio-volume-muted-symbolic";
    if (percentage <= 33) return "audio-volume-low-symbolic";
    if (percentage <= 66) return "audio-volume-medium-symbolic";
    return "audio-volume-high-symbolic";
  };

  try {
    currentIcon.value = await getSymbolSource(getIconName());
  } catch (error) {
    console.error("Error loading icon:", error);
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

async function getVolumeInfo() {
  try {
    const info: any = await invoke("get_audio_volume");
    volumeInfo.value = info;
    currentVolume.value = info.current;
    await updateIcon();
  } catch (error) {
    console.error("Error getting volume:", error);
  }
}

async function toggleMute() {
  try {
    const isUnmuted = await invoke("toggle_audio_mute");
    volumeInfo.value.is_muted = !isUnmuted;
    await getVolumeInfo();
    await updateIcon();
  } catch (error) {
    console.error("Error toggling mute:", error);
  }
}

onMounted(async () => {
  await getVolumeInfo();
});
</script>
