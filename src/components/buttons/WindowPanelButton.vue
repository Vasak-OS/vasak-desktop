<script setup lang="ts">
import { ref, onMounted, Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getIconSource } from '@vasakgroup/plugin-vicons';

interface Props {
  id: string;
  title: string;
  is_minimized: boolean;
  icon: string;
}

const props = defineProps<Props>();
const iconSource: Ref<string> = ref<string>('');

const toggleWindow = async (): Promise<void> => {
	try {
		await invoke('toggle_window', { windowId: props.id });
	} catch (error) {
		console.error('[Window Error] Error alternando ventana:', error);
	}
};

onMounted(async () => {
	if (props.icon) {
		iconSource.value = await getIconSource(props.icon);
	}
});
</script>

<template>
  <div
    class="window-button"
    :class="{ 'opacity-50 hover:opacity-90': is_minimized }"
    @click="toggleWindow"
  >
    <img
      v-if="icon && iconSource"
      :src="iconSource"
      :alt="title"
      :title="title"
      class="w-6 h-6 transition-all duration-300 group-hover:rotate-3 group-hover:brightness-110"
    />
    <div v-else class="w-6 h-6 bg-gray-500/50 rounded-vsk animate-pulse" />
  </div>
</template>

<style>
@reference "../../style.css";

.window-button {
  @apply flex items-center justify-center w-7 h-7 cursor-pointer transform rounded-vsk hover:bg-vsk-primary/30 hover:scale-110 active:scale-95 relative;
}

.window-minimized {
  opacity: 0.5;
}

.window-icon {
  width: 24px;
  height: 24px;
}

.icon-placeholder {
  width: 24px;
  height: 24px;
  background-color: rgba(128, 128, 128, 0.5);
  border-radius: 6px;
}
</style>
