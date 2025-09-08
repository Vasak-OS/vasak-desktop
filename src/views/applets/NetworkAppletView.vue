<script lang="ts" setup>
import NetworkControlArea from "@/components/areas/network/NetworkControlArea.vue";
import { invoke } from "@tauri-apps/api/core";
import { onMounted } from "vue";

onMounted(async () => {
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      try {
        invoke("toggle_network_applet");
      } catch (error) {
        console.error("Error al cerrar:", error);
      }
    }
  });
});
</script>

<template>
  <div class="applet background h-screen w-screen">
    <NetworkControlArea />
  </div>
</template>