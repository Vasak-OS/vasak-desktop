<script lang="ts" setup>
import { ref, onMounted, Ref } from "vue";
import { getIconSource } from "@vasakgroup/plugin-vicons";
import { invoke } from "@tauri-apps/api/core";

const props = defineProps({
  app: {
    type: Object,
    required: true,
  },
});

const appIcon: Ref<string> = ref(props.app.icon);

const openApp = async () => {
  try {
    await invoke("open_app", { path: props.app.path });
  } catch (error) {
    console.error("Error al abrir la aplicación:", error);
  }
};

const getAppIcon = async () => {
  appIcon.value = await getIconSource(props.app.icon);
};

onMounted(() => {
  getAppIcon();
});
</script>

<template>
  <button
    :title="app.name"
    @click="openApp()"
    class="transform hover:translate-y-1 hover:scale-110 transition-transform duration-200"
  >
    <img :src="appIcon" class="h-10 m-2" :alt="app.name" :title="app.name" />
    <span style="display: none">{{ app.name }}</span>
    <span style="display: none">{{ app.description }}</span>
    <span style="display: none">{{ app.keywords }}</span>
  </button>
</template>
