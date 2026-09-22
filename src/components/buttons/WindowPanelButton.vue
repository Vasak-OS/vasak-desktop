<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import type { WindowPanelButtonProps } from '@/interfaces/window';
import { toggleWindow as sysToggleWindow } from '@/services/window.service';
import { logError } from '@/utils/logger';

const props = defineProps<WindowPanelButtonProps>();
const iconName = computed(() => props.icon?.trim() || 'application-x-executable');

const toggleWindow = async (): Promise<void> => {
	try {
		await sysToggleWindow({ windowId: props.id });
	} catch (error) {
		logError('[Window] Error alternando ventana:', error);
	}
};
</script>

<template>
  <button
    type="button"
    class="theme-transition flex items-center justify-center w-7 h-7 cursor-pointer transform rounded-corner hover:bg-primary/30 hover:scale-110 active:scale-95 relative"
    :class="{ 'opacity-50 hover:opacity-90': is_minimized }"
    :title="title"
    :aria-label="title"
    @click="toggleWindow"
  >
    <!-- El latido de antes se va: `ThemeIcon` ya reserva el hueco del mismo
         tamaño mientras resuelve, que es para lo que servía, y aquel también
         aparecía cuando el tema **no tiene** el icono —un estado permanente
         pulsando como si algo estuviera por llegar—. -->
    <ThemeIcon
      :name="iconName"
      :size="24"
      :alt="title"
      class="transition-all duration-300 group-hover:rotate-3 group-hover:brightness-110"
    />
  </button>
</template>

