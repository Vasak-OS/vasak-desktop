import { openApp } from '@/services/app.service';

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
});

const appIcon = useIcon(computed(() => props.app.icon));
const appWindow = getCurrentWindow();

const openApp = async (path: string) => {
	try {
		await sysOpenApp({ path } as any);
	} catch (error) {
		logError('Error al abrir aplicación:', error);
	} finally {
		appWindow.close();
	}
};
</script>

<template>
  <button
    class="flex flex-row w-full p-2 rounded-corner items-center transform hover:translate-x-1 hover:scale-110 hover:bg-primary hover:border hover:border-secondary transition-transform"
    @click="openApp(app.path)"
    :title="app.description"
  >
    <img
      :src="appIcon"
      :alt="app.name"
      :title="app.name"
      class="img-fluid h-10"
    />
    <div class="col-10 app-card-info ps-2 text-left">
      {{ app.name }}
      <span class="text-muted" style="display: none">{{
        app.description
      }}</span>
      <span style="display: none">{{ app.keywords }}</span>
    </div>
  </button>
</template>
