<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { TrayIconButton } from '@vasakgroup/vue-libvasak';
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
  <!-- El botón del panel de la librería, el mismo de la bandeja: el velo
       neutro al pasar y el anillo de foco por dentro, sin crecer ni girar.
       Minimizada, la ventana se dibuja atenuada. -->
  <TrayIconButton
    :name="iconName"
    :alt="title"
    :tooltip="title"
    :icon-class="{ 'opacity-50': Boolean(is_minimized) }"
    @click="toggleWindow"
  />
</template>
