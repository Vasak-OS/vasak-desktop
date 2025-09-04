<script lang="ts" setup>
import DesktopClockWidget from "@/components/widgets/DesktopClockWidget.vue";
import MusicWidget from "@/components/widgets/MusicWidget.vue";
import { ref, Ref } from "vue";

const background: Ref<string> = ref(
  "https://png.pngtree.com/background/20250102/original/pngtree-planetary-glow-stunning-celestial-view-pc-wallpaper-picture-image_15736228.jpg"
);
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
