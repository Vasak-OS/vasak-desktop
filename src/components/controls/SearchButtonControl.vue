<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { onMounted, Ref, ref } from "vue";
import { getIconSource } from "@vasakgroup/plugin-vicons";

const iconSrc: Ref<string> = ref("");

const openSearch = async () => {
  try {
    await invoke("toggle_search");
  } catch (error) {
    console.error("Error opening search:", error);
  }
};

onMounted(async () => {
  iconSrc.value = await getIconSource("search");
});
</script>

<template>
  <button
    @click="openSearch"
    class="p-2 rounded-vsk background transition-all duration-500 h-[70px] w-[70px] group relative overflow-hidden hover:scale-105 active:scale-95 ring-2 ring-vsk-primary/50"
    title="Open Global Search"
  >
    <!-- Overlay decorativo como ThemeToggle -->
    <div
      class="absolute inset-0 rounded-vsk transition-all duration-500"
      :class="'bg-gradient-to-br from-vsk-primary/20 to-vsk-primary/10'"
      style="opacity: 0"
    ></div>

    <img
      :src="iconSrc"
      alt="Search"
      class="m-auto w-12 h-12 transition-all duration-500 group-hover:scale-110 relative z-10 drop-shadow-lg group-hover:drop-shadow-xl"
    />
  </button>
</template>
