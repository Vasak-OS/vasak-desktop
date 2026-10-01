<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ProgressBar, TrayIconButton } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import type { WindowPanelButtonProps } from '@/interfaces/window';
import { toggleWindow as sysToggleWindow } from '@/services/window.service';
import { countLabel, progressPercent } from '@/tools/tray-item';
import { logError } from '@/utils/logger';

const props = defineProps<WindowPanelButtonProps>();
const { t } = useI18n();
const iconName = computed(() => props.icon?.trim() || 'application-x-executable');

/** El contador de la aplicación, sólo si lo hace visible y es mayor que cero. */
const badge = computed(() =>
	countLabel(props.launcher?.count) ? (props.launcher?.count ?? null) : null
);
/** El progreso, en porcentaje, sólo con `progress-visible`. */
const progress = computed(() => progressPercent(props.launcher?.progress));

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
    :badge="badge"
    :icon-class="{ 'opacity-50': Boolean(is_minimized) }"
    @click="toggleWindow"
  >
    <!-- El progreso que publica la aplicación (una descarga, una copia), sólo
         si lo hace visible. Sin progreso no hay barra. -->
    <div
      v-if="progress !== undefined"
      data-window-progress
      class="pointer-events-none absolute inset-x-1 bottom-0"
    >
      <ProgressBar :value="progress" :label="t('components.tray.progress').replace('{0}', title)" />
    </div>
  </TrayIconButton>
</template>
