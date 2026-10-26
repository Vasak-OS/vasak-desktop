
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
import { useSharedEvent } from '@/tools/event.bus';
import { launcherForApp } from '@/tools/tray-item';
import {
	scrollWindowsPillOnWheel,
	windowsPillCapStyle,
	windowsPillClasses,
} from '@/tools/windows-pill';
import { logError } from '@/utils/logger';

interface WindowDelta {
	added: WindowInfo[];
	removed: string[];
	modified: WindowInfo[];
}

// De costado las ventanas se apilan, y lo que sobra scrollea a lo largo de la
// barra en vez de desbordarse fuera de la pantalla.
//
// La densidad elegida decide además si la píldora reserva su ancho (compacta,
// vasak-desktop#213) o si cede a las vecinas (distribuida, #161): ver la nota
// del `<template>`.
const { vertical, layout, hasSurface } = usePanelConfig();
const { t } = useI18n();

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

/**
 * La rueda desplaza la píldora en su eje (horizontal, o vertical de costado): sin
 * esto, una rueda vertical sobre el desbordamiento horizontal no alcanza las
 * ventanas de más. `currentTarget` es la propia píldora, que es el contenedor que
 * desplaza. Ver `scrollWindowsPillOnWheel`.
 */
const onWheel = (event: WheelEvent): void => {
	const node = event.currentTarget as HTMLElement;
	if (scrollWindowsPillOnWheel(node, vertical.value, event.deltaY, event.deltaX)) {
		event.preventDefault();
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
       una píldora más, y sin ventanas no se dibuja. No se pliega en ningún
       ancho (decisión del usuario, 03/10/2026): son sólo iconos, y lo que no
       entra se desplaza adentro de la píldora.

       En densidad **distribuida** se acota para no pisar a las vecinas
       (vasak-desktop#161): en horizontal, un ancho máximo (`max-w-[20rem]`) y
       `min-w-0` sin `shrink-0`, así cuando falta lugar la fila cede y lo que no
       entra se desplaza adentro —`overflow` en el eje de la barra— en vez de
       empujar al resto. La barra, de borde a borde, le reparte el hueco con
       pistas `1fr`, así que con lugar se ven todas igual.

       En densidad **compacta** la barra se encoge a su contenido (`w-fit`), y
       ahí la píldora se quedaba en su mínimo —una sola ventana con scroll
       adentro— aunque sobrara lugar: la barra no le reservaba ancho. Por eso en
       compacto lleva `shrink-0` (vasak-desktop#213) **y no** el `max-w-[20rem]`:
       reserva su ancho y usa el que la barra le dé. El viejo tope `max-w-[90vw]`
       dejaba crecer la píldora casi hasta la pantalla y recién ahí desplazaba,
       así que con muchas ventanas quedaba una tira larguísima, cortada sin poder
       desplazar cómodo. Ahora se fija a **tres iconos** —`windowsPillCapStyle`,
       un ancho en `rem` sacado del tamaño real del botón, el relleno y el borde
       (vasak-desktop#251)— y lo que sobra se desplaza adentro (`overflow-x-auto`).
       El tope en línea gana sobre el `max-w-full` de la propia `PanelPill`, que
       sigue cuidando el ancho angosto (240/360 px) cuando no hay tope de iconos.
       De costado la píldora toma el alto que sobra (`flex-1`) pero se fija igual a
       tres iconos de **alto** (`max-height` en línea) y desplaza hacia abajo lo
       que sobra (`overflow-y-auto`). En densidad distribuida sigue cediendo a las
       vecinas hasta `max-w-[20rem]`, sin tope de iconos. La barra de
       desplazamiento va oculta (`.windows-pill`, abajo): se ve una píldora limpia,
       se arrastra igual con la rueda o el gesto. -->
  <PanelPill
    v-if="windows.length > 0"
    :interactive="false"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    :flat="hasSurface"
    flush
    role="group"
    :accessible-label="t('views.panel.windowsAlt')"
    class="windows-pill min-w-0"
    :class="windowsPillClasses(vertical, layout)"
    :style="windowsPillCapStyle(vertical, layout)"
    data-windows-pill
    v-on="{ wheel: onWheel }"
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

<style scoped>
/**
 * La píldora de ventanas desplaza su contenido sin barra visible
 * (vasak-desktop#161): la barra global de `scrollbar.css` dibujaría 8 píxeles
 * dentro de la píldora y la afearía. Van las dos formas: el pseudoelemento de
 * WebKit —lo que dibuja WebKitGTK y el banco en Chromium— y `scrollbar-width`
 * estándar. El desplazamiento sigue: rueda, gesto táctil y teclado.
 */
.windows-pill {
	scrollbar-width: none;
}

.windows-pill::-webkit-scrollbar {
	display: none;
}
</style>
