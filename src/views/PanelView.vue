<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { showContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { Badge, PanelPill, ThemeIcon } from '@vasakgroup/vue-libvasak';
import {
	computed,
	defineAsyncComponent,
	nextTick,
	onBeforeUnmount,
	onMounted,
	ref,
	watch,
} from 'vue';

const TrayBarArea = defineAsyncComponent(() => import('@/components/areas/panel/TrayBarArea.vue'));
const TrayMusicControl = defineAsyncComponent(
	() => import('@/components/controls/TrayMusicControl.vue')
);
const TrayWeatherControl = defineAsyncComponent(
	() => import('@/components/controls/TrayWeatherControl.vue')
);

import WindowsArea from '@/components/areas/panel/WindowsArea.vue';
import TrayIconBattery from '@/components/buttons/TrayIconBattery.vue';
import TrayIconBluetooth from '@/components/buttons/TrayIconBluetooth.vue';
import TrayIconNetwork from '@/components/buttons/TrayIconNetwork.vue';
import TrayIconSound from '@/components/buttons/TrayIconSound.vue';
import KeyboardLayoutPill from '@/components/panel/KeyboardLayoutPill.vue';
import PinnedAppsPill from '@/components/panel/PinnedAppsPill.vue';
import WorkspacesPill from '@/components/panel/WorkspacesPill.vue';
import PanelClockWidget from '@/components/widgets/PanelClockWidget.vue';
import type { ConnectDevice } from '@/interfaces/connect';
import type {
	Notification as AppNotification,
	NotificationDelta,
} from '@/interfaces/notifications';
import { listConnectDevices, toggleConnectMenu } from '@/services/connect.service';
import { getAllNotifications } from '@/services/notification.service';
import { reportMenuButton, toggleControlCenter, toggleMenu } from '@/services/window.service';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
import { usePanelAutohide } from '@/tools/composables/usePanelAutohide';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { usePanelDensity, watchPanelDensity } from '@/tools/composables/usePanelDensity';
import { usePanelInputRegion } from '@/tools/composables/usePanelInputRegion';
import { usePanelTrapezoidClip } from '@/tools/composables/usePanelTrapezoidClip';
import { useSharedEvent } from '@/tools/event.bus';
import { containsNewNotifications } from '@/tools/notifications';
import { panelHideClass, panelRegionMode } from '@/tools/panel-autohide';
import { GROUP_CLASSES } from '@/tools/panel-position';
import { logError } from '@/utils/logger';

const { t } = useI18n();

/**
 * De qué lado va el panel, y por lo tanto cómo se acomoda lo de adentro.
 *
 * El backend ancla la superficie al borde que diga la configuración; acá se
 * decide el sentido de la fila. A los costados la barra mide 38 píxeles de
 * ancho y todo pasa a ser una columna: una fila de iconos ahí no entra.
 *
 * Es reactivo, así que mover el panel en Configuración lo acomoda en el acto,
 * al mismo tiempo que la superficie se reancla.
 */
const {
	position,
	vertical,
	style,
	showWeather,
	showMusic,
	showWindows,
	showWorkspaces,
	barClasses,
	surfaceClass,
	hasSurface,
	animationClass,
	autohide,
} = usePanelConfig();

const windowsFirst = computed(() => density.value === 'tight');

/**
 * Esconder y revelar la barra cuando `panel.autohide` está puesto. La máquina de
 * estados vive aparte (`usePanelAutohide`); `panelHidden` desliza la barra y
 * recorta la región de entrada. El puntero entrando y saliendo de la franja lo
 * avisa el backend por `panel-pointer-entered` / `panel-pointer-left` —la página
 * no recibe la salida del puntero en una superficie de capa— y se conectan a
 * revelar y esconder.
 */
const {
	hidden: panelHidden,
	reveal: revealPanel,
	requestHide: hidePanel,
} = usePanelAutohide(autohide, position);
useSharedEvent('panel-pointer-entered', () => revealPanel());
useSharedEvent('panel-pointer-left', () => hidePanel());

/**
 * Lo que recibe el puntero, según el estado (`panel-autohide.ts`): sólo las
 * píldoras (lo de siempre), la barra entera (con superficie o ya revelada), o la
 * línea del borde (escondida). Entre píldoras se ve el escritorio y un clic ahí
 * tiene que caer en él; con superficie o revelada no hay huecos.
 */
const regionMode = computed(() =>
	panelRegionMode({
		autohide: autohide.value,
		hidden: panelHidden.value,
		hasSurface: hasSurface.value,
	})
);

const bar = ref<HTMLElement | null>(null);
usePanelInputRegion(bar, regionMode, position);

/**
 * Densidad del panel: si el usuario la cambia, ajustamos el CSS del
 * contenedor y la sombra de la superficie.
 */
usePanelDensity();

/**
 * Vigila los cambios de densidad en Configuración y los replica en la
 * variable CSS `--panel-density` para que los estilos reaccionen.
 */
watchPanelDensity();

/**
 * El clip trapezoidal del panel según la posición y el estado de autohide.
 */
usePanelTrapezoidClip(position, vertical, animationClass, barClasses, panelHidden);

/**
 * Conexiones móviles (KDE Connect / GSConnect).
 */
const connectDevices = ref<ConnectDevice[]>([]);
const connectLoading = ref(false);
const connectError = ref<string | null>(null);

async function cargarDispositivos() {
	if (connectLoading.value) return;
	connectLoading.value = true;
	connectError.value = null;
	try {
		connectDevices.value = await listConnectDevices();
	} catch (error: any) {
		connectError.value = error?.message ?? 'Error desconocido';
		logError('Error al listar dispositivos Connect', { error: error?.message });
	} finally {
		connectLoading.value = false;
	}
}

/**
 * Notificaciones.
 */
const notifications = ref<AppNotification[]>([]);
const notificationsLoading = ref(false);
const notificationsError = ref<string | null>(null);
const MAX_NOTIFICATIONS = 10;

async function cargarNotificaciones() {
	if (notificationsLoading.value) return;
	notificationsLoading.value = true;
	notificationsError.value = null;
	try {
		const lista = await getAllNotifications();
		notifications.value = lista.slice(0, MAX_NOTIFICATIONS);
	} catch (error: any) {
		notificationsError.value = error?.message ?? 'Error desconocido';
		logError('Error al cargar notificaciones', { error: error?.message });
	} finally {
		notificationsLoading.value = false;
	}
}

const hasUnreadNotifications = computed(() => notifications.value.some((n) => !n.read));

/**
 * Menú de notificaciones: se abre con el icono de la campana.
 */
const showNotificationsMenu = ref(false);
const notificationsMenuAnchor = ref<HTMLElement | null>(null);

function toggleNotificationsMenu() {
	showNotificationsMenu.value = !showNotificationsMenu.value;
}

function closeNotificationsMenu() {
	showNotificationsMenu.value = false;
}

/**
 * Marcador de audio (micrófono) — usa el composable que aísla la lógica del
 * icono y el texto.
 */
const { icon: micIcon, label: micLabel, active: micActive } = useOpenApplet('mic', t);

/**
 * Marcador de pantalla (captura) — igual que el de audio.
 */
const { icon: screenIcon, label: screenLabel, active: screenActive } = useOpenApplet('screen', t);

/**
 * Menú contextual del reloj.
 */
const showClockMenu = ref(false);
const clockMenuAnchor = ref<HTMLElement | null>(null);

function toggleClockMenu() {
	showClockMenu.value = !showClockMenu.value;
}

function closeClockMenu() {
	showClockMenu.value = false;
}

/**
 * Menú contextual del usuario.
 */
const showUserMenu = ref(false);
const userMenuAnchor = ref<HTMLElement | null>(null);

function toggleUserMenu() {
	showUserMenu.value = !showUserMenu.value;
}

function closeUserMenu() {
	showUserMenu.value = false;
}

/**
 * Menú contextual de KDE Connect.
 */
const showConnectMenu = ref(false);
const connectMenuAnchor = ref<HTMLElement | null>(null);

function toggleConnectMenuPanel() {
	if (showConnectMenu.value) {
		closeConnectMenuPanel();
	} else {
		cargarDispositivos();
		showConnectMenu.value = true;
	}
}

function closeConnectMenuPanel() {
	showConnectMenu.value = false;
}

/**
 * Atajos de teclado globales.
 */
onMounted(() => {
	cargarNotificaciones();
	cargarDispositivos();

	const handleKeydown = (event: KeyboardEvent) => {
		// Meta+Space: menú de aplicaciones
		if (event.metaKey && event.code === 'Space') {
			event.preventDefault();
			toggleMenu();
		}
		// Meta+Shift+A: control center
		if (event.metaKey && event.shiftKey && event.code === 'KeyA') {
			event.preventDefault();
			toggleControlCenter();
		}
		// Meta+Shift+N: notificaciones
		if (event.metaKey && event.shiftKey && event.code === 'KeyN') {
			event.preventDefault();
			toggleNotificationsMenu();
		}
	};

	window.addEventListener('keydown', handleKeydown);
	onBeforeUnmount(() => {
		window.removeEventListener('keydown', handleKeydown);
	});
});

/**
 * Eventos compartidos.
 */
useSharedEvent('connect-devices-changed', () => cargarDispositivos());
useSharedEvent('notifications-changed', () => cargarNotificaciones());
useSharedEvent('panel-pointer-entered', () => revealPanel());
useSharedEvent('panel-pointer-left', () => hidePanel());
</script>

<template>
  <div
    ref="bar"
    :class="[
      'flex fixed z-50',
      barClasses[position],
      animationClass,
      panelHidden.value ? 'translate-y-full' : 'translate-y-0',
    ]"
    :style="surfaceClass"
    data-panel-bar
  >
    <div
      class="flex w-full items-center gap-1.5 px-2"
      :class="GROUP_CLASSES[position].start"
      data-panel-left
    >
      <PinnedAppsPill />
      <Suspense>
        <TrayMusicControl v-if="showMusic" />
        <template #fallback><div class="w-10 h-10 animate-pulse bg-ui-line" /></template>
      </Suspense>
      <WindowsArea v-if="showWindows && windowsFirst" />
      <WorkspacesPill v-if="showWorkspaces" />
    </div>
    <div class="flex items-center gap-1.5" :class="GROUP_CLASSES[position].center" data-panel-center>
      <PanelClockWidget />
      <Suspense>
        <TrayWeatherControl v-if="showWeather" />
        <template #fallback><div class="w-10 h-10 animate-pulse bg-ui-line" /></template>
      </Suspense>
    </div>
    <div class="flex min-w-0 items-center gap-1.5 overflow-x-clip" :class="GROUP_CLASSES[position].end" data-panel-end>
      <Suspense>
        <TrayBarArea />
        <template #fallback><div class="w-10 h-10 animate-pulse bg-ui-line" /></template>
      </Suspense>
      <KeyboardLayoutPill />
      <TrayIconNetwork />
      <TrayIconBluetooth />
      <TrayIconSound />
      <TrayIconBattery />
    </div>
  </div>

  <!-- Menú de notificaciones -->
  <div
    v-if="showNotificationsMenu"
    class="fixed z-50"
    :style="{ top: '48px', right: '16px' }"
  >
    <div class="bg-ui-surface border border-ui-border style="border-radius: var(--radius-corner-lg)" shadow-surface-l w-80 max-h-96 overflow-hidden">
      <div class="flex items-center justify-between px-4 py-3 border-b border-ui-border">
        <span class="font-medium">{{ t('panel.notifications') }}</span>
        <button
          @click="closeNotificationsMenu"
          class="p-1 style="border-radius: var(--radius-corner-m)" hover:bg-ui-hover transition-colors"
          aria-label="Cerrar"
        >
          <ThemeIcon name="close" size="20" />
        </button>
      </div>
      <div class="max-h-[400px] overflow-y-auto">
        <div v-if="notificationsLoading" class="p-8 text-center text-tx-muted">
          Cargando…
        </div>
        <div v-else-if="notificationsError" class="p-4 text-center text-status-error">
          {{ notificationsError }}
        </div>
        <div v-else-if="notifications.length === 0" class="p-8 text-center text-tx-muted">
          {{ t('panel.noNotifications') }}
        </div>
        <div v-else class="divide-y divide-ui-border">
          <div
            v-for="notification in notifications"
            :key="notification.id"
            class="p-4 hover:bg-ui-hover transition-colors"
          >
            <div class="flex items-start gap-3">
              <ThemeIcon :name="notification.icon" size="24" class="text-tx-main shrink-0" />
              <div class="flex-1 min-w-0">
                <p class="font-medium truncate">{{ notification.title }}</p>
                <p class="text-sm text-tx-muted truncate">{{ notification.body }}</p>
                <p class="text-xs text-tx-muted mt-1">{{ notification.timestamp }}</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Menú del reloj -->
  <div
    v-if="showClockMenu"
    class="fixed z-50"
    :style="{ top: '48px', right: '16px' }"
  >
    <div class="bg-ui-surface border border-ui-border style="border-radius: var(--radius-corner-lg)" shadow-surface-l w-56">
      <div class="p-2">
        <button
          @click="closeClockMenu"
          class="w-full flex items-center gap-2 px-3 py-2 style="border-radius: var(--radius-corner-m)" hover:bg-ui-hover transition-colors"
        >
          <ThemeIcon name="calendar-month" size="20" />
          <span>{{ t('panel.calendar') }}</span>
        </button>
        <button
          @click="closeClockMenu"
          class="w-full flex items-center gap-2 px-3 py-2 style="border-radius: var(--radius-corner-m)" hover:bg-ui-hover transition-colors"
        >
          <ThemeIcon name="clock" size="20" />
          <span>{{ t('panel.dateTime') }}</span>
        </button>
      </div>
    </div>
  </div>

  <!-- Menú de usuario -->
  <div
    v-if="showUserMenu"
    class="fixed z-50"
    :style="{ top: '48px', right: '16px' }"
  >
    <div class="bg-ui-surface border border-ui-border style="border-radius: var(--radius-corner-lg)" shadow-surface-l w-56">
      <div class="p-2">
        <button
          @click="closeUserMenu"
          class="w-full flex items-center gap-2 px-3 py-2 style="border-radius: var(--radius-corner-m)" hover:bg-ui-hover transition-colors"
        >
          <ThemeIcon name="account" size="20" />
          <span>{{ t('panel.userSettings') }}</span>
        </button>
        <button
          @click="closeUserMenu"
          class="w-full flex items-center gap-2 px-3 py-2 style="border-radius: var(--radius-corner-m)" hover:bg-ui-hover transition-colors"
        >
          <ThemeIcon name="lock" size="20" />
          <span>{{ t('panel.lock') }}</span>
        </button>
        <button
          @click="closeUserMenu"
          class="w-full flex items-center gap-2 px-3 py-2 style="border-radius: var(--radius-corner-m)" hover:bg-ui-hover transition-colors text-status-error"
        >
          <ThemeIcon name="logout" size="20" />
          <span>{{ t('panel.logout') }}</span>
        </button>
      </div>
    </div>
  </div>

  <!-- Menú de KDE Connect -->
  <div
    v-if="showConnectMenu"
    class="fixed z-50"
    :style="{ top: '48px', right: '16px' }"
  >
    <div class="bg-ui-surface border border-ui-border style="border-radius: var(--radius-corner-lg)" shadow-surface-l w-72 max-h-96 overflow-hidden">
      <div class="flex items-center justify-between px-4 py-3 border-b border-ui-border">
        <span class="font-medium">{{ t('panel.kdeConnect') }}</span>
        <button
          @click="closeConnectMenuPanel"
          class="p-1 style="border-radius: var(--radius-corner-m)" hover:bg-ui-hover transition-colors"
          aria-label="Cerrar"
        >
          <ThemeIcon name="close" size="20" />
        </button>
      </div>
      <div class="max-h-[400px] overflow-y-auto">
        <div v-if="connectLoading" class="p-8 text-center text-tx-muted">
          Cargando…
        </div>
        <div v-else-if="connectError" class="p-4 text-center text-status-error">
          {{ connectError }}
        </div>
        <div v-else-if="connectDevices.length === 0" class="p-8 text-center text-tx-muted">
          {{ t('panel.noDevices') }}
        </div>
        <div v-else class="divide-y divide-ui-border">
          <div
            v-for="device in connectDevices"
            :key="device.id"
            class="p-4 hover:bg-ui-hover transition-colors"
          >
            <div class="flex items-center gap-3">
              <ThemeIcon :name="device.icon" size="24" class="shrink-0" />
              <div class="flex-1 min-w-0">
                <p class="font-medium truncate">{{ device.name }}</p>
                <p class="text-sm text-tx-muted truncate">{{ device.type }}</p>
              </div>
              <button
                @click="() => toggleConnectMenu(device.id)"
                class="px-3 py-1 text-sm style="border-radius: var(--radius-corner-m)" bg-primary text-on-primary hover:bg-primary-hover transition-colors"
              >
                {{ device.connected ? t('panel.disconnect') : t('panel.connect') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
