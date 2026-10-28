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
	RevealTransition,
	SlideTransition,
	ThemeIcon,
	ToggleControl,
} from '@vasakgroup/vue-libvasak';
import { computed, nextTick, onBeforeUnmount, onMounted, type Ref, ref, watch } from 'vue';
import CalendarSheet from '@/components/areas/control-center/CalendarSheet.vue';
import NotificationArea from '@/components/areas/control-center/NotificationArea.vue';
import QuickSettingsPanel from '@/components/areas/control-center/QuickSettingsPanel.vue';
import SessionActionsRow from '@/components/areas/control-center/SessionActionsRow.vue';
import PhoneControlCenterCard from '@/components/cards/PhoneControlCenterCard.vue';
import UserControlCenterCard from '@/components/cards/UserControlCenterCard.vue';
import BluetoothControl from '@/components/controls/BluetoothControl.vue';
import BrightnessControl from '@/components/controls/BrightnessControl.vue';
import DoNotDisturbToggle from '@/components/controls/DoNotDisturbToggle.vue';
import MicrophoneControl from '@/components/controls/MicrophoneControl.vue';
import MonitorBrightnessList from '@/components/controls/MonitorBrightnessList.vue';
import NetworkControl from '@/components/controls/NetworkControl.vue';
import NightLightToggle from '@/components/controls/NightLightToggle.vue';
import PowerProfileControl from '@/components/controls/PowerProfileControl.vue';
import ThemeToggle from '@/components/controls/ThemeToggle.vue';
import VolumeControl from '@/components/controls/VolumeControl.vue';
import MusicWidget from '@/components/widgets/MusicWidget.vue';
import { hideControlCenter } from '@/services/window.service';
import {
	type ControlCenterMode,
	modeOnCountChange,
	modeOnOpen,
	summaryText,
} from '@/tools/control-center-mode';
import type { SheetId } from '@/tools/control-center-sheets';
import type { TileId } from '@/tools/control-center-tiles';
import { useSharedEvent } from '@/tools/event.bus';

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
const detail = ref<TileId | SheetId | null>(null);

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

/**
 * La flecha › del volumen o del micrófono: la elección del dispositivo se abre
 * dentro del bloque de ajustes, y desde A primero se pasa a B.
 */
function openSheet(id: SheetId): void {
	mode.value = 'settings';
	detail.value = id;
}

/** La línea resumen: vuelven las notificaciones y se cierra cualquier ficha. */
function showNotifications(): void {
	detail.value = null;
	calendarOpen.value = false;
	mode.value = 'notifications';
}

// ── El calendario ────────────────────────────────────────────────────────────

/**
 * Si el calendario del mes está abierto (vasak-desktop#183). Se abre tocando
 * la fecha de la tarjeta de usuario y tapa lo de abajo —notificaciones o
 * ajustes— como una ficha con «Volver»; al volver queda el estado que había.
 */
const calendarOpen = ref(false);
const userCard = ref<{ $el?: Element } | null>(null);

function openCalendar(): void {
	calendarOpen.value = true;
}

/** Vuelve con el foco en la fecha que lo abrió. */
async function closeCalendar(): Promise<void> {
	calendarOpen.value = false;
	await nextTick();
	userCard.value?.$el?.querySelector<HTMLElement>('[data-user-date]')?.focus();
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
	calendarOpen.value = false;
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
  <RevealTransition v-bind="{ appear: true }" from="right">
    <main
      v-show="!leaving"
      :class="['bg-ui-shell h-screen w-screen rounded-corner-m flex flex-col justify-between p-1 window-border overflow-hidden']"
      :data-mode="mode"
    >
      <!-- La cabecera —quién sos, la sesión y el teléfono— queda fija arriba:
           no se desliza aunque la grilla de ajustes o las notificaciones no
           entren. Lo que se desliza es sólo el escenario de abajo, así nunca se
           mueve la parte con tu foto. -->
      <div class="flex min-h-0 flex-1 flex-col w-full gap-2 p-2" data-top>
        <div class="flex shrink-0 flex-col gap-2" data-header>
          <UserControlCenterCard ref="userCard" :calendar-open="calendarOpen" @open-calendar="openCalendar" />
          <!-- La sesión, debajo de quién sos: Bloquear, Cerrar sesión, Reiniciar,
               Apagar. Bloquear no pregunta; las demás, con el diálogo de
               sesión. -->
          <SessionActionsRow class="shrink-0 justify-end" />
          <PhoneControlCenterCard />
        </div>

        <!-- El escenario del cross-slide: A (notificaciones) y B (ajustes) son dos
             paneles lado a lado en un carril del doble de ancho que se traslada
             —-50 % en B— mientras los dos se deslizan a la vez. Los dos paneles
             siempre están montados (la lista de notificaciones no se desmonta y
             es la que cuenta para la línea resumen); el que no se ve queda
             `inert`, fuera del foco y de los lectores. Con movimiento reducido
             el cambio es instantáneo: la transición se apaga, el transform
             queda. El calendario del mes entra como una página aparte que
             reemplaza el escenario (ver abajo); lo de abajo sigue montado. -->
        <div class="relative min-h-0 flex-1 overflow-hidden" data-stage>
          <!-- El calendario del mes es una página más del centro: entra y sale
               con el deslizamiento de la casa (`SlideTransition`, §5.5), no
               apareciendo de golpe encima. Mientras se ve, el carril de abajo
               queda `invisible` además de `inert` —sigue montado, así las
               notificaciones no se vuelven a pedir, pero no se transparenta
               debajo del mes—, de modo que la página no se superpone con lo que
               había. -->
          <SlideTransition direction="right">
            <CalendarSheet
              v-if="calendarOpen"
              class="absolute inset-0 z-10 overflow-y-auto"
              @back="closeCalendar"
            />
          </SlideTransition>

          <div
            :class="[
              'flex h-full w-full transition-transform duration-200 ease-ui-out motion-reduce:transition-none',
              showsSettings ? '-translate-x-full' : 'translate-x-0',
              calendarOpen ? 'invisible' : '',
            ]"
            :inert="calendarOpen ? true : undefined"
            data-track
          >
            <!-- A: la lista, con el alto entero que sobra y nunca menos que unas
                 dos tarjetas (`min-h-40`), la fila de interruptores redondos con
                 «más» y el brillo. `min-h-0` + `overflow-y-auto` desplazan la
                 lista; los controles de abajo quedan quietos. -->
            <section
              class="flex min-h-0 w-full shrink-0 flex-col gap-2"
              :inert="showsSettings ? true : undefined"
              data-panel="notifications"
            >
              <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto" data-scroll>
                <NotificationArea
                  class="min-h-40 flex-1"
                  data-notifications
                  @summary="onSummary"
                />
              </div>

              <div class="shrink-0 p-2">
                <!-- Los interruptores redondos y «más». Se parten en renglones en
                     vez de salirse cuando el centro es angosto. En B están en los
                     mosaicos. -->
                <div class="flex w-full flex-wrap justify-between gap-2" data-quick-toggles>
                  <NetworkControl />
                  <BluetoothControl v-if="bluetoothInitialized" />
                  <DoNotDisturbToggle />
                  <NightLightToggle />
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
                  <!-- Un solo brillo, el del monitor principal. -->
                  <div data-primary-brightness-box>
                    <BrightnessControl />
                  </div>
                </div>
              </div>
            </section>

            <!-- B: las notificaciones en una línea que las vuelve a abrir (nunca
                 desaparecen) y el bloque de mosaicos; debajo, el micrófono y el
                 brillo por monitor con el perfil de energía. Los mosaicos no
                 existen hasta que B se ve por primera vez con el centro
                 abierto. -->
            <section
              class="flex min-h-0 w-full shrink-0 flex-col gap-2"
              :inert="showsSettings ? undefined : true"
              data-panel="settings"
            >
              <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto" data-scroll>
                <ListRow
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
                  v-model:detail="detail"
                  class="flex-1"
                  :bluetooth="bluetoothInitialized"
                />
              </div>

              <div class="shrink-0 p-2">
                <div class="flex flex-col gap-2 w-full mt-4">
                  <!-- El micrófono, componente propio. -->
                  <MicrophoneControl
                    v-if="tilesMounted"
                    @open-devices="openSheet('audio-input')"
                  />
                  <!-- Un brillo por monitor y el perfil de energía (issue 189 de
                       vasak-desktop). -->
                  <div v-if="tilesMounted" class="flex flex-col gap-2" data-display-power>
                    <MonitorBrightnessList />
                    <PowerProfileControl />
                  </div>
                </div>
              </div>
            </section>
          </div>
        </div>
      </div>

      <!-- La barra de abajo, compartida por A y B: la música y el volumen no
           pueden vivir en el carril que se desliza. -->
      <div class="flex shrink-0 flex-wrap w-full justify-around items-end p-2">
        <!-- La caja va afuera y no como clase del widget: por dentro es
             `h-full` —se adapta con consultas de contenedor— y una clase de
             alto puesta desde acá compite con esa y pierde. Sin una altura
             resuelta, la carátula se estira hasta tapar el volumen.

             `v-show` y no `v-if`: el widget es quien escucha a MPRIS y avisa
             si hay algo sonando, así que tiene que seguir montado aunque no se
             vea. Escondido no ocupa lugar y las notificaciones crecen. -->
        <div v-show="musicActive" class="h-24 w-full" data-music-box>
          <MusicWidget class="w-full" @presence="musicActive = $event" />
        </div>
        <div class="flex flex-col gap-2 w-full mt-4">
          <VolumeControl devices @open-devices="openSheet('audio-output')" />
        </div>
      </div>
    </main>
  </RevealTransition>
</template>
