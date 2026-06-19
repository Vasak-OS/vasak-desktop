
<script lang="ts" setup>
import { getCurrentWindow } from '@tauri-apps/api/window';
import { computed } from 'vue';
import { useIcon } from '@/tools/composables/useReactiveIcon';
import { openApp as sysOpenApp } from '@/services/app.service';
import { logError } from '@/utils/logger';

const props = defineProps({
	app: {
		type: Object,
		required: true,
	},
	selected: {
		type: Boolean,
		default: false,
	},
});

const appIcon = useIcon(computed(() => props.app.icon));
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
</script>

<template>
  <button
    :title="app.name"
    @click="openApp()"
    :class="[
      'transform hover:translate-y-1 hover:scale-110 transition-transform duration-200 rounded-corner',
      selected ? 'bg-primary/20 border border-secondary scale-110' : ''
    ]"
  >
    <img :src="appIcon" class="h-10 m-2" :alt="app.name" :title="app.name" />
    <span style="display: none">{{ app.name }}</span>
    <span style="display: none">{{ app.description }}</span>
    <span style="display: none">{{ app.keywords }}</span>
  </button>
</template>
