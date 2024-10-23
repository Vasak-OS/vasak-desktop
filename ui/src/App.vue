<script lang="ts" setup>
import { onMounted, Ref, ref } from "vue";
import ClockWidget from "./components/widgets/ClockWidgert.vue";
//import background from "./assets/background.mp4";

const background: Ref<string> = ref("/home/pato/.backgroud.mp4");

const setBackground = async (): Promise<void> => {
  background.value =
    (await (this as any).$vsk.getHome()) + "/.config/vsk/background.mp4";
};

onMounted(() => {
  setBackground();
});
</script>

<template>
  <div class="global">
    <video
      style="border-radius: 0px"
      id="video-background"
      type="video/mp4"
      :src="background"
      loop
      autoplay
      muted
    ></video>
    <ClockWidget />
  </div>
</template>

<style>
body,
#app,
.global {
  overflow: auto;
  width: 100vw;
  height: 100vh;
}
</style>
