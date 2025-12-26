<template>
  <div
    class="background rounded-vsk flex flex-row items-center gap-2 justify-between w-full h-auto p-4 transition-all duration-200 hover:bg-white/60 dark:hover:bg-black/60"
  >
    <button
      class="w-8 h-8 flex items-center justify-center rounded-vsk transition-all duration-200 hover:bg-white/30 dark:hover:bg-black/30 hover:scale-110 active:scale-95"
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
import { ref, computed, onMounted, onUnmounted, Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getSymbolSource } from "@vasakgroup/plugin-vicons";

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
let unlisten: (() => void) | null = null;
let setDebitTimeout: ReturnType<typeof setTimeout> | null = null;

async function updateIcon() {
  try {
    const iconName =
      brightnessPercentage.value > 66
        ? "display-brightness-high-symbolic"
        : brightnessPercentage.value > 33
        ? "display-brightness-medium-symbolic"
        : "display-brightness-low-symbolic";

    currentIcon.value = await getSymbolSource(iconName);
  } catch (error) {
    console.error("Error loading brightness icon:", error);
  }
}

const brightnessPercentage = computed(() => {
  if (brightnessInfo.value.max <= 0) return 0;
  // Calculate percentage from raw value
  const range = brightnessInfo.value.max - brightnessInfo.value.min;
  const loading = currentBrightness.value - brightnessInfo.value.min;
  return Math.round((loading / range) * 100);
});

async function getBrightnessInfo() {
  try {
    const info = await invoke("get_brightness_info") as BrightnessInfo;
    brightnessInfo.value = info;
    currentBrightness.value = info.current;
    await updateIcon();
  } catch (error) {
    console.error("Error getting brightness:", error);
  }
}

// Debounce the outgoing update to avoid flooding the backend/hardware
async function updateBrightness() {
  if (setDebitTimeout) clearTimeout(setDebitTimeout);
  
  // Optimistic update of UI
  await updateIcon();

  setDebitTimeout = setTimeout(async () => {
    try {
      await invoke("set_brightness_info", {
        brightness: Number(currentBrightness.value),
      });
    } catch (error) {
      console.error("Error setting brightness:", error);
    }
  }, 50); // Short debounce for responsiveness
}

onMounted(async () => {
  await getBrightnessInfo();

  unlisten = await listen("brightness-changed", (event: any) => {
    const payload = event.payload; 
    // Payload: { value: number, max: number, percentage: number }
    if (payload) {
      // Avoid loop if the value is close to what we just set
      // We check raw value difference.
      if (Math.abs(currentBrightness.value - payload.value) > 1) {
          currentBrightness.value = payload.value;
          // Update max if changed (adaptive max?)
          if (payload.max > 0) brightnessInfo.value.max = payload.max;
          brightnessInfo.value.current = payload.value;
          updateIcon();
      }
    }
  });
});

onUnmounted(() => {
  if (unlisten) unlisten();
  if (setDebitTimeout) clearTimeout(setDebitTimeout);
});
</script>

<style scoped>
@reference '../../style.css';

/* Estilos personalizados para el slider de brillo */
input[type="range"] {
  -webkit-appearance: none;
  appearance: none;
  cursor: pointer;
  background: rgba(234, 179, 8, 0.2);
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
