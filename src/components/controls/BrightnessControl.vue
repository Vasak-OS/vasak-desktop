<template>
  <div
    class="bg-white/50 dark:bg-black/50 rounded-xl flex flex-row items-center gap-2 justify-between w-full h-auto p-4 transition-all duration-200 hover:bg-white/60 dark:hover:bg-black/60"
  >
    <button
      class="w-8 h-8 flex items-center justify-center rounded-lg transition-all duration-200 hover:bg-white/30 dark:hover:bg-black/30 hover:scale-110 active:scale-95"
    >
      <img
        :src="currentIcon"
        :alt="'Brillo: ' + brightnessPercentage + '%'"
        class="w-6 h-6 transition-all duration-200"
      />
    </button>
    <input
      type="range"
      :min="brightnessInfo.min"
      :max="brightnessInfo.max"
      v-model="currentBrightness"
      @input="updateBrightness"
      class="flex-1 transition-all duration-200 hover:scale-105"
    />
    <span
      class="w-12 text-right transition-all duration-200 font-medium"
      :class="{
        'text-yellow-500': brightnessPercentage > 80,
        'text-orange-500': brightnessPercentage < 20,
      }"
      >{{ brightnessPercentage }}%</span
    >
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getIconSource } from "@vasakgroup/plugin-vicons";

interface BrightnessInfo {
  current: number;
  min: number;
  max: number;
}

const brightnessInfo: Ref<BrightnessInfo> = ref<BrightnessInfo>({
  current: 100,
  min: 0,
  max: 100,
});

const currentBrightness: Ref<number> = ref(100);
const currentIcon: Ref<string> = ref("");

async function updateIcon() {
  try {
    const iconName =
      brightnessPercentage.value > 66
        ? "display-brightness-high-symbolic"
        : brightnessPercentage.value > 33
        ? "display-brightness-medium-symbolic"
        : "display-brightness-low-symbolic";

    currentIcon.value = await getIconSource(iconName);
  } catch (error) {
    console.error("Error loading brightness icon:", error);
  }
}

const brightnessPercentage = computed(() => {
  return Math.round(currentBrightness.value);
});

async function getBrightnessInfo() {
  try {
    const info = await invoke("get_brightness");
    brightnessInfo.value = info as any;
    currentBrightness.value = (info as any).current;
    await updateIcon();
  } catch (error) {
    console.error("Error getting brightness:", error);
  }
}

async function updateBrightness() {
  try {
    await invoke("set_brightness", {
      brightness: Number(currentBrightness.value),
    });
    await updateIcon();
  } catch (error) {
    console.error("Error setting brightness:", error);
  }
}

onMounted(async () => {
  await getBrightnessInfo();
});
</script>

<style scoped>
@reference '../../style.css';

/* Estilos personalizados para el slider de brillo */
input[type="range"] {
  -webkit-appearance: none;
  appearance: none;
  cursor: pointer;
  height: 6px;
  border-radius: 9999px;
}

/* Track del slider */
input[type="range"]::-webkit-slider-track {
  background: #eab308;
  height: 6px;
  border-radius: 9999px;
  transition: all 0.2s ease;
}

/* Thumb del slider */
input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  height: 20px;
  width: 20px;
  border-radius: 50%;
  background: #eab308;
  cursor: pointer;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15), 0 0 0 1px rgba(234, 179, 8, 0.2);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

/* Hover effects */
input[type="range"]:hover::-webkit-slider-thumb {
  transform: scale(1.25);
  box-shadow: 0 4px 16px rgba(234, 179, 8, 0.4),
    0 0 0 4px rgba(234, 179, 8, 0.1);
  background: #ca8a04;
}

input[type="range"]:active::-webkit-slider-thumb {
  transform: scale(1.35);
  box-shadow: 0 2px 12px rgba(234, 179, 8, 0.6),
    0 0 0 6px rgba(234, 179, 8, 0.2);
  background: #a16207;
}
</style>
