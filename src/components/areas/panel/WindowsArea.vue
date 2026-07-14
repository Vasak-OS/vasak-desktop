
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
import { onMounted, ref } from 'vue';
import WindowPanelButton from '@/components/buttons/WindowPanelButton.vue';
import type { WindowInfo } from '@/interfaces/window';
import { getWindows } from '@/services/window.service';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

interface WindowDelta {
	added: WindowInfo[];
	removed: string[];
	modified: WindowInfo[];
}

const windows = ref<WindowInfo[]>([]);

const refreshWindows = async (): Promise<void> => {
	try {
		windows.value = await getWindows();
	} catch (error) {
		logError('[Windows] Error obteniendo ventanas:', error);
	}
};

const applyDelta = (delta: WindowDelta): void => {
	try {
		// Remove windows by ID
		if (delta.removed.length > 0) {
			const removedSet = new Set(delta.removed);
			windows.value = windows.value.filter((w) => !removedSet.has(w.id));
		}

		// Update modified windows in-place
		for (const modified of delta.modified) {
			const index = windows.value.findIndex((w) => w.id === modified.id);
			if (index !== -1) {
				windows.value[index] = modified;
			}
		}

		// Add new windows
		if (delta.added.length > 0) {
			windows.value.push(...delta.added);
		}
	} catch (error) {
		logError('[Windows] Error applying delta, falling back to full refetch:', error);
		refreshWindows();
	}
};

onMounted(async () => {
	await refreshWindows();
});

useSharedEvent<WindowDelta>('window-delta', applyDelta);
</script>

<template>
  <div class="flex items-center justify-center px-3 overflow-x-auto overflow-y-hidden">
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

