
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { getCurrentWindow } from '@tauri-apps/api/window';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { onMounted, type Ref, ref } from 'vue';
import { openApp as sysOpenApp } from '@/services/app.service';
import { logError } from '@/utils/logger';

const props = defineProps({
	app: {
		type: Object,
		required: true,
	},
});

const appIcon: Ref<string> = ref(props.app.icon);
const appWindow = getCurrentWindow();

const openApp = async () => {
	try {
		await sysOpenApp({ path: props.app.path } as any);
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
