
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { PanelPill } from '@vasakgroup/vue-libvasak';
import { onMounted, ref } from 'vue';
import WindowPanelButton from '@/components/buttons/WindowPanelButton.vue';
import type { LauncherEntryView } from '@/interfaces/tray';
import type { WindowInfo } from '@/interfaces/window';
import { getLauncherEntries } from '@/services/tray.service';
import { getWindows } from '@/services/window.service';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { usePanelDensity } from '@/tools/composables/usePanelDensity';
import { useSharedEvent } from '@/tools/event.bus';
import { showsNumbers } from '@/tools/panel-density';
import { launcherForApp } from '@/tools/tray-item';
import { logError } from '@/utils/logger';

interface WindowDelta {
	added: WindowInfo[];
	removed: string[];
	modified: WindowInfo[];
}

// De costado las ventanas se apilan, y lo que sobra scrollea a lo largo de la
// barra en vez de desbordarse fuera de la pantalla.
const { vertical } = usePanelConfig();
const { t } = useI18n();
const density = usePanelDensity();

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

/**
 * Lo que publican las aplicaciones por `LauncherEntry` (contador, progreso):
 * se dibuja sobre su ventana. Sólo llega lo visible.
 */
const launcherEntries = ref<LauncherEntryView[]>([]);

const refreshLauncherEntries = async (): Promise<void> => {
	try {
		launcherEntries.value = await getLauncherEntries();
	} catch (error) {
		logError('[Windows] Error obteniendo LauncherEntry:', error);
	}
};

onMounted(async () => {
	await refreshWindows();
	await refreshLauncherEntries();
});

useSharedEvent<WindowDelta>('window-delta', applyDelta);
useSharedEvent<LauncherEntryView[]>('launcher-entry-update', (entries) => {
	launcherEntries.value = Array.isArray(entries) ? entries : [];
});
</script>

<template>
  <!-- Las ventanas abiertas, en su píldora del panel (vasak-desktop#151). El
       video de referencia no tiene barra de tareas, pero acá es la única
       manera de volver a una ventana minimizada sin abrir el menú: queda, en
       una píldora más, y sin ventanas no se dibuja. En un panel angosto se
       pliega: las ventanas siguen en el menú. Lo que no entra se
       desplaza adentro de la píldora en vez de empujar a las demás. -->
  <PanelPill
    v-if="windows.length > 0 && showsNumbers(density)"
    :interactive="false"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    flush
    role="group"
    :accessible-label="t('views.panel.windowsAlt')"
    class="min-w-0"
    :class="vertical
      ? 'min-h-0 flex-1 py-1 overflow-y-auto overflow-x-hidden'
      : 'px-1 overflow-x-auto overflow-y-hidden'"
    data-windows-pill
  >
    <TransitionGroup
      move-class="transition-transform duration-300 ease-in-out" enter-active-class="transition-all duration-300 ease-in-out" leave-active-class="transition-all duration-300 ease-in-out" enter-from-class="opacity-0 translate-y-[30px]" leave-to-class="opacity-0 translate-y-[30px]"
      tag="div"
      class="flex items-center gap-0.5"
      :class="vertical ? 'flex-col' : ''"
    >
      <WindowPanelButton
        v-for="window in windows"
        :key="window.id"
        v-bind="window"
        :launcher="launcherForApp(launcherEntries, window.app_id)"
      />
    </TransitionGroup>
  </PanelPill>
</template>
