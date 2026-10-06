<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
/**
 * El centro de control, con sus dos estados (vasak-desktop#175, ancla #174).
 *
 * - **A, notificaciones:** la lista con todo el alto que sobra (y un mínimo de
 *   unas dos tarjetas), la fila de interruptores redondos con «más», el brillo
 *   y el volumen.
 * - **B, ajustes:** las notificaciones resumidas en una línea que las vuelve a
 *   abrir, y el bloque de mosaicos (`QuickSettingsPanel`) en su lugar.
 *
 * Abre en A si hay notificaciones y en B si no; no se recuerda entre
 * aperturas. La conmutación es este estado local y CSS (`v-show`): nada se
 * vuelve a pedir al cambiar. Los mosaicos de B no existen —ni su código ni sus
 * vigilancias— hasta que B se ve por primera vez con el centro abierto.
 */
import { isBluetoothPluginInitialized } from '@vasakgroup/plugin-bluetooth-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ListRow,
	type PowerAction,
	PowerActions,
	ThemeIcon,
	ToggleControl,
} from '@vasakgroup/vue-libvasak';
import { computed, onBeforeUnmount, onMounted, type Ref, ref, watch } from 'vue';
import NotificationArea from '@/components/areas/control-center/NotificationArea.vue';
import QuickSettingsPanel from '@/components/areas/control-center/QuickSettingsPanel.vue';
import PhoneControlCenterCard from '@/components/cards/PhoneControlCenterCard.vue';
import UserControlCenterCard from '@/components/cards/UserControlCenterCard.vue';
import BluetoothControl from '@/components/controls/BluetoothControl.vue';
import BrightnessControl from '@/components/controls/BrightnessControl.vue';
import NetworkControl from '@/components/controls/NetworkControl.vue';
import ThemeToggle from '@/components/controls/ThemeToggle.vue';
import VolumeControl from '@/components/controls/VolumeControl.vue';
import MusicWidget from '@/components/widgets/MusicWidget.vue';
import { hideControlCenter, toggleSessionPopup } from '@/services/window.service';
import {
	type ControlCenterMode,
	modeOnCountChange,
	modeOnOpen,
	summaryText,
} from '@/tools/control-center-mode';
import type { TileId } from '@/tools/control-center-tiles';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const bluetoothInitialized: Ref<boolean> = ref(false);
/**
 * Si hay un reproductor sonando o en pausa (vasak-desktop#176). Lo avisa el
 * propio widget; sin reproductor la caja se esconde y su alto vuelve a las
 * notificaciones.
 */
const musicActive = ref(false);
const leaving = ref(false);

// ── Los dos estados ──────────────────────────────────────────────────────────

const mode = ref<ControlCenterMode>('notifications');
const showsSettings = computed(() => mode.value === 'settings');

/** Lo que cuenta la lista de notificaciones (lo avisa `NotificationArea`). */
const summary = ref({ count: 0, apps: 0, loaded: false });
const summaryLabel = computed(() => summaryText(t, summary.value.count, summary.value.apps));

/** Si el centro está a la vista. Empieza escondido: se crea al iniciar la sesión. */
const visible = ref(false);
/** Abrió antes de que llegara la primera cuenta: decide cuando llegue. */
let pendingDecision = false;

/** Los mosaicos se montan la primera vez que B se ve, y ya no se desmontan. */
const tilesMounted = ref(false);
/** La ficha abierta dentro del bloque de ajustes, si hay una. */
const detail = ref<TileId | null>(null);

watch(
	[showsSettings, visible],
	([settings, shown]) => {
		if (settings && shown) tilesMounted.value = true;
	},
	{ immediate: true }
);

function onSummary(next: { count: number; apps: number; loaded: boolean }): void {
	const previous = summary.value.count;
	summary.value = next;
	if (!next.loaded) return;
	if (pendingDecision) {
		pendingDecision = false;
		mode.value = modeOnOpen(next.count);
		return;
	}
	if (visible.value) mode.value = modeOnCountChange(previous, next.count, mode.value);
}

/** «Más»: el bloque de ajustes crece y las notificaciones quedan en una línea. */
function showSettings(): void {
	mode.value = 'settings';
}

/** La línea resumen: vuelven las notificaciones y se cierra cualquier ficha. */
function showNotifications(): void {
	detail.value = null;
	mode.value = 'notifications';
}

// ── La sesión ────────────────────────────────────────────────────────────────

/** Las acciones del diálogo de sesión, que es también su confirmación. */
const SESSION_ACTIONS: readonly PowerAction[] = ['suspend', 'logout', 'reboot', 'poweroff'];

const sessionLabels = computed(() => ({
	suspend: t('views.menu.suspend'),
	logout: t('views.menu.logout'),
	reboot: t('views.menu.reboot'),
	poweroff: t('views.menu.shutdown'),
}));

/**
 * Abre el diálogo de sesión con la acción elegida, que pregunta antes de
 * hacerla —el mismo camino que los botones del menú—. El diálogo toma el foco
 * y el centro se cierra solo al perderlo.
 */
async function onSessionAction(action: PowerAction): Promise<void> {
	try {
		await toggleSessionPopup(action === 'poweroff' ? 'shutdown' : action);
	} catch (error) {
		logError('[control-center] no se pudo abrir el diálogo de sesión:', error);
	}
}

/** Track timeout handles for cleanup */
let closeTimeout: ReturnType<typeof setTimeout> | null = null;

const closeAfterAnimation = () => {
	if (leaving.value) return; // Prevent double-close
	leaving.value = true;

	// Cancel entrance animation if still running
	const main = document.querySelector('main');
	main?.getAnimations().forEach((a) => {
		a.cancel();
	});

	// Play exit animation, then close window.
	//
	// Hides rather than toggles, and that is the whole bug: pressing the panel
	// button closed the centre and brought it straight back. The click reaches
	// the panel and toggles the centre shut, while this timer — armed by the
	// blur the same click raised — fires 200 ms later, finds it shut, and calls
	// it open again. The menu had this verbatim.
	closeTimeout = setTimeout(() => {
		visible.value = false;
		hideControlCenter().catch(() => {
			/* window already closed */
		});
	}, 200);
};

const onKeydown = (event: KeyboardEvent) => {
	if (event.key === 'Escape') {
		closeAfterAnimation();
	}
};

const onBlur = () => {
	closeAfterAnimation();
};

// Shown again after being hidden. The window is never destroyed, so Vue does
// not re-run and the animation state survives from the last dismissal — without
// this the centre comes back mid-fade and `leaving` keeps closeAfterAnimation
// short-circuited for good.
useSharedEvent('window-shown', () => {
	if (closeTimeout !== null) {
		clearTimeout(closeTimeout);
		closeTimeout = null;
	}
	leaving.value = false;
	visible.value = true;
	detail.value = null;
	// No se recuerda entre aperturas: decide si hay notificaciones.
	if (summary.value.loaded) mode.value = modeOnOpen(summary.value.count);
	else pendingDecision = true;
});

onMounted(async () => {
	bluetoothInitialized.value = await isBluetoothPluginInitialized();
	document.addEventListener('keydown', onKeydown);
	window.addEventListener('blur', onBlur);
});

onBeforeUnmount(() => {
	// Clear all timeouts
	if (closeTimeout !== null) {
		clearTimeout(closeTimeout);
		closeTimeout = null;
	}
	// Remove all event listeners registered during mount
	document.removeEventListener('keydown', onKeydown);
	window.removeEventListener('blur', onBlur);
});
</script>

<template>
  <Transition appear enter-active-class="enter-active">
    <main
      :class="['bg-ui-shell h-screen w-screen rounded-corner-m flex flex-col justify-between p-1 border border-ui-line overflow-hidden', { 'leave-active': leaving }]"
      :data-mode="mode"
    >
      <!-- `min-h-0` es lo que hace que el scroll sea de las notificaciones y no
           del centro entero: sin él, un hijo flexible no se deja achicar por
           debajo de su contenido, la lista empujaba y los controles de abajo
           —música, brillo, volumen— se iban de la pantalla en cuanto había unas
           cuantas notificaciones. La lista tiene además un mínimo de unas dos
           tarjetas (`min-h-40`); en una pantalla tan baja que ese mínimo no
           entra, este bloque se desplaza (`overflow-y-auto`) en vez de pisar a
           los controles de abajo. -->
      <div class="flex min-h-0 flex-1 flex-col w-full gap-2 overflow-y-auto p-2" data-top>
        <UserControlCenterCard />
        <!-- La sesión, debajo de quién sos: las acciones del diálogo de
             sesión, que pregunta antes de hacerlas. -->
        <PowerActions
          class="shrink-0 justify-end"
          :actions="SESSION_ACTIONS"
          :labels="sessionLabels"
          :label="t('views.controlCenter.session')"
          button-variant="ghost"
          data-session-actions
          @action="onSessionAction"
        />
        <PhoneControlCenterCard />

        <!-- A: la lista, con el alto entero que sobra y nunca menos que unas dos
             tarjetas. Escondida con `v-show` en B: sigue montada, y es la que
             cuenta para la línea resumen. -->
        <NotificationArea
          v-show="!showsSettings"
          class="min-h-40 flex-1"
          data-notifications
          @summary="onSummary"
        />

        <!-- B: las notificaciones en una línea que las vuelve a abrir. Nunca
             desaparecen. -->
        <ListRow
          v-if="showsSettings"
          class="shrink-0"
          role="button"
          icon="preferences-desktop-notification"
          icon-type="symbol"
          :title="summaryLabel"
          aria-expanded="false"
          truncate
          data-notification-summary
          @click="showNotifications"
        >
          <template #trailing>
            <ThemeIcon name="go-down" type="symbol" :size="16" alt="" />
          </template>
        </ListRow>

        <QuickSettingsPanel
          v-if="tilesMounted"
          v-show="showsSettings"
          v-model:detail="detail"
          class="flex-1"
          :bluetooth="bluetoothInitialized"
        />
      </div>
      <div class="flex shrink-0 flex-wrap w-full justify-around items-end p-2">
        <!-- La caja va afuera y no como clase del widget: por dentro es
             `h-full` —se adapta con consultas de contenedor— y una clase de
             alto puesta desde acá compite con esa y pierde. Sin una altura
             resuelta, la carátula se estira hasta tapar el brillo y el
             volumen.

             `v-show` y no `v-if`: el widget es quien escucha a MPRIS y avisa
             si hay algo sonando, así que tiene que seguir montado aunque no se
             vea. Escondido no ocupa lugar y las notificaciones crecen. -->
        <div v-show="musicActive" class="h-24 w-full" data-music-box>
          <MusicWidget class="w-full" @presence="musicActive = $event" />
        </div>
        <!-- A: los interruptores redondos y «más». Se parten en renglones en
             vez de salirse cuando el centro es angosto. En B están en los
             mosaicos. -->
        <div
          v-show="!showsSettings"
          class="flex w-full flex-wrap justify-between gap-2"
          data-quick-toggles
        >
          <NetworkControl />
          <BluetoothControl v-if="bluetoothInitialized" />
          <ThemeToggle />
          <ToggleControl
            name="go-up"
            type="symbol"
            :label="t('views.controlCenter.more')"
            aria-expanded="false"
            aria-controls="control-center-quick-settings"
            data-more
            @click="showSettings"
          />
        </div>
        <div class="flex flex-col gap-2 w-full mt-4">
          <BrightnessControl />
          <VolumeControl />
        </div>
      </div>
    </main>
  </Transition>
</template>

<style scoped>
@keyframes slide-in-right {
  from {
    transform: translateX(100%);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

@keyframes slide-out-right {
  from {
    transform: translateX(0);
    opacity: 1;
  }
  to {
    transform: translateX(100%);
    opacity: 0;
  }
}

.enter-active {
  animation: slide-in-right 200ms ease-out;
}

.leave-active {
  animation: slide-out-right 200ms ease-in;
}
</style>
