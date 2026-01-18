<script lang="ts" setup>
import DesktopClockWidget from "@/components/widgets/DesktopClockWidget.vue";
import MusicWidget from "@/components/widgets/MusicWidget.vue";
import { ref, Ref, computed, onMounted, onUnmounted } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const backgroundPath: Ref<string> = ref(
  "/usr/share/backgrounds/cutefishos/wallpaper-9.jpg"
);
const background = computed(() => convertFileSrc(backgroundPath.value));
const backgroundType: Ref<string> = ref("image/jpeg");

let unlistenConfigChanged: (() => void) | null = null;

// Cargar configuración desde el plugin
const loadConfig = async () => {
  try {
    const configStr = await invoke<string>("plugin:config-manager|read_config");
    const config = JSON.parse(configStr);
    
    // Leer wallpaper desde config.desktop.wallpaper
    if (config.desktop?.wallpaper && config.desktop.wallpaper.length > 0) {
      backgroundPath.value = config.desktop.wallpaper[0];
      
      // Detectar tipo de archivo
      const ext = backgroundPath.value.toLowerCase().split('.').pop();
      if (ext === 'mp4' || ext === 'webm' || ext === 'ogv') {
        backgroundType.value = `video/${ext}`;
      } else {
        backgroundType.value = `image/${ext || 'jpeg'}`;
      }
    }
  } catch (error) {
    console.error("Error loading config:", error);
  }
};

onMounted(async () => {
  await loadConfig();
  const appWindow = getCurrentWindow();
  unlistenConfigChanged = await appWindow.listen("config-changed", async () => {
    await loadConfig();
  });
});

onUnmounted(() => {
  if (unlistenConfigChanged) {
    unlistenConfigChanged();
  }
});
</script>

<template>
  <video
    v-if="backgroundType.includes('video')"
    style="border-radius: 0px"
    :type="backgroundType"
    :src="background"
    class="w-screen h-screen object-cover absolute z-10"
    loop
    autoplay
    muted
  ></video>
  <img
    v-else
    :src="background"
    alt="Background"
    class="w-screen h-screen object-cover absolute z-10"
    style="border-radius: 0px"
  />
  <main
    class="w-screen h-screen flex flex-col items-center justify-center absolute z-20"
  >
    <MusicWidget />
    <DesktopClockWidget />
  </main>
</template>
