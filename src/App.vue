<script setup lang="ts">
import { ref } from "vue";

const background = ref("https://png.pngtree.com/background/20250102/original/pngtree-planetary-glow-stunning-celestial-view-pc-wallpaper-picture-image_15736228.jpg");
const backgroundType = ref("image/jpeg");


</script>

<template>
  <main class="w-screen h-screen flex items-center justify-center">
    <video v-if="backgroundType.includes('video')" style="border-radius: 0px" :type="backgroundType" :src="background"
      class="w-screen h-screen object-cover" loop autoplay muted></video>
    <img v-else :src="background" class="w-screen h-screen object-cover" style="border-radius: 0px" />
  </main>
</template>
