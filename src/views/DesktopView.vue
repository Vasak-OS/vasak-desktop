<script lang="ts" setup>
import DesktopClockWidget from "@/components/widgets/DesktopClockWidget.vue";
import MusicWidget from "@/components/widgets/MusicWidget.vue";
import { ref, Ref, computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";

const backgroundPath: Ref<string> = ref(
  "/usr/share/backgrounds/cutefishos/wallpaper-9.jpg"
);
const background = computed(() => convertFileSrc(backgroundPath.value));
const backgroundType: Ref<string> = ref("image/jpeg");
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
