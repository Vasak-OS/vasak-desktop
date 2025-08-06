<script setup lang="ts">
import { RouterView } from "vue-router";
import { useConfigStore } from "@vasakgroup/plugin-config-manager";
import { listen } from "@tauri-apps/api/event";
import { onMounted, onUnmounted } from "vue";

const configStore = useConfigStore();
let unlistenConfig: Function | null = null;

onMounted(async () => {
  configStore.loadConfig();
  unlistenConfig = await listen("config-changed", async () => {
    configStore.loadConfig();
  });
});

onUnmounted(() => {
  if (unlistenConfig !== null) {
    unlistenConfig();
  }
});
</script>

<template>
  <RouterView />
</template>
