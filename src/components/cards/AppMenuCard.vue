<script lang="ts" setup>
import { ref, onMounted, Ref } from 'vue';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';

const props = defineProps({
	app: {
		type: Object,
		required: true,
	},
});

const appIcon: Ref<string> = ref(props.app.icon);
const appWindow = getCurrentWindow();

const openApp = async (path: string) => {
	try {
		await invoke('open_app', { path });
	} catch (error) {
		logError('Error al abrir aplicación:', error);
	} finally {
		appWindow.close();
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
    class="flex flex-row w-full p-2 rounded-vsk items-center transform hover:translate-x-1 hover:scale-110 hover:bg-vsk-primary/50 transition-transform"
    @click="openApp(app.path)"
    :title="app.description"
  >
    <img
      :src="appIcon"
      :alt="app.name"
      :title="app.name"
      class="img-fluid h-10"
    />
    <div class="col-10 app-card-info ps-2">
      {{ app.name }}
      <span class="text-muted" style="display: none">{{
        app.description
      }}</span>
      <span style="display: none">{{ app.keywords }}</span>
    </div>
  </button>
</template>
