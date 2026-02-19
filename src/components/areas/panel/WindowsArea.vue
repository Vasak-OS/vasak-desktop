<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { onMounted, onUnmounted, ref } from 'vue';
import WindowPanelButton from '@/components/buttons/WindowPanelButton.vue';
import type { WindowInfo } from '@/interfaces/window';
import { logError } from '@/utils/logger';

const windows = ref<WindowInfo[]>([]);
let unlisten: (() => void) | null = null;

const refreshWindows = async (): Promise<void> => {
	try {
		windows.value = await invoke('get_windows');
	} catch (error) {
		logError('[Windows] Error obteniendo ventanas:', error);
	}
};

onMounted(async () => {
	await refreshWindows();
	unlisten = await listen('window-update', refreshWindows);
});

onUnmounted(() => {
	unlisten?.();
});
</script>

<template>
  <div class="flex items-center justify-center flex-grow overflow-x-auto overflow-y-hidden scrollbar-thin scrollbar-thumb-white/20 scrollbar-track-transparent">
    <TransitionGroup 
      name="window-list"
      tag="div"
      class="flex items-center justify-center gap-0.5"
    >
      <WindowPanelButton
        v-for="window in windows"
        :key="window.id"
        v-bind="window"
      />
    </TransitionGroup>
  </div>
</template>

<style >
.window-list-move {
  transition: transform 0.3s ease;
}
.window-list-enter-active,
.window-list-leave-active {
  transition: all 0.3s ease;
}
.window-list-enter-from,
.window-list-leave-to {
  opacity: 0;
  transform: translateY(30px);
}
</style>
