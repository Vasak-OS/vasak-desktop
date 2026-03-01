
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { onMounted, onUnmounted, ref } from 'vue';
import WindowPanelButton from '@/components/buttons/WindowPanelButton.vue';
import type { WindowInfo } from '@/interfaces/window';
import { getWindows } from '@/services/window.service';
import { logError } from '@/utils/logger';

const windows = ref<WindowInfo[]>([]);
let unlisten: (() => void) | null = null;

const refreshWindows = async (): Promise<void> => {
	try {
		windows.value = await getWindows();
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
      move-class="transition-transform duration-300 ease-in-out" enter-active-class="transition-all duration-300 ease-in-out" leave-active-class="transition-all duration-300 ease-in-out" enter-from-class="opacity-0 translate-y-[30px]" leave-to-class="opacity-0 translate-y-[30px]"
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

