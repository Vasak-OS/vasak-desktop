<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { Command } from '@tauri-apps/plugin-shell';
import { showContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import TrayBarArea from '@/components/areas/panel/TrayBarArea.vue';
import WindowsArea from '@/components/areas/panel/WindowsArea.vue';
import PanelClockwidget from '@/components/widgets/PanelClockwidget.vue';
import type { ConnectDevice } from '@/interfaces/connect';
import type {
	Notification as AppNotification,
	NotificationDelta,
} from '@/interfaces/notifications';
import { listConnectDevices, toggleConnectMenu } from '@/services/connect.service';
import { getAllNotifications } from '@/services/notification.service';
import { toggleControlCenter, toggleMenu } from '@/services/window.service';
import { useIcons } from '@/tools/composables/useReactiveIcon';
import { useSharedEvent } from '@/tools/event.bus';
import { hayNotificacionesNuevas } from '@/tools/notificaciones';
import { logError } from '@/utils/logger';

const { t } = useI18n();

/**
 * El clic derecho del panel: sólo cosas del panel.
 *
 * Lo que se hace con el escritorio —los widgets, el fondo— tiene su propio clic
 * derecho ahí, y ofrecerlo también acá daría dos caminos para lo mismo, uno de
 * ellos en el lugar equivocado.
 *
 * Va en modo ventana porque el panel mide unos treinta píxeles de alto: un menú
 * dibujado adentro quedaría recortado a la primera línea. El resto —el dibujo,
 * el teclado, el tema, cerrarse al perder el foco— es del plugin, que es el
 * mismo menú que usan todas las aplicaciones de VasakOS.
 */
const abrirMenuDelPanel = async (evento: MouseEvent) => {
	try {
		const elegido = await showContextMenu(
			[
				{
					id: 'panel',
					label: t('views.applets.panelMenu.panelSettings'),
					icon: 'preferences-system-windows',
				},
				{
					id: 'notificaciones',
					label: t('views.applets.panelMenu.notifications'),
					icon: 'preferences-desktop-notification',
				},
				{ type: 'separator' },
				{
					id: 'sistema',
					label: t('views.applets.panelMenu.systemSettings'),
					icon: 'preferences-system',
				},
			],
			evento,
			{ window: true }
		);

		switch (elegido?.id) {
			case 'panel':
				await invoke('open_settings_section', { section: 'appearance-panel' });
				break;
			case 'notificaciones':
				await toggleControlCenter();
				break;
			case 'sistema':
				await invoke('open_settings');
				break;
		}
	} catch (error) {
		logError('No se pudo abrir el menú del panel:', error);
	}
};

const notifications = ref<AppNotification[]>([]);
const hasNewNotifications = ref(false);
let notificationResetTimer: ReturnType<typeof setTimeout> | undefined;

const { menuIcon, notifyIcon, configIcon, fileManagerIcon, phoneIcon } = useIcons({
	menuIcon: 'start-here',
	notifyIcon: 'preferences-desktop-notification',
	configIcon: 'preferences-system',
	fileManagerIcon: 'system-file-manager',
	phoneIcon: 'smartphone',
});

/**
 * Phones the device service can see.
 *
 * The button only exists while there is one. A permanent icon for a feature
 * that needs hardware most people never plug in is clutter in the one strip of
 * screen that is always visible.
 */
const connectDevices = ref<ConnectDevice[]>([]);

const hasPhone = computed(() => connectDevices.value.length > 0);

/**
 * True while a phone is plugged in but nobody has accepted the debugging
 * prompt yet. Worth a mark on the icon: from the outside it looks identical to
 * a phone that simply has no apps.
 */
const phoneNeedsAuth = computed(() =>
	connectDevices.value.some((device) => device.state === 'unauthorized')
);

const refreshConnectDevices = async () => {
	connectDevices.value = await listConnectDevices();
};

const openPhoneMenu = async () => {
	try {
		await toggleConnectMenu();
	} catch (error) {
		logError('Error al abrir el menú del teléfono:', error);
	}
};

const openMenu = async () => {
	try {
		await toggleMenu();
	} catch (error) {
		logError('Error al abrir el menu:', error);
	}
};

const openConfig = async () => {
	try {
		const cmd = Command.create('vasak-settings', []);
		await cmd.spawn();
	} catch (error) {
		logError('Error al abrir config:', error);
	}
};

const openFileManager = async () => {
	try {
		const cmd = Command.create('vasak-file-manager', []);
		await cmd.spawn();
	} catch (error) {
		logError('Error al abrir file manager:', error);
	}
};

const openNotificationCenter = async () => {
	try {
		await toggleControlCenter();
	} catch (error) {
		logError('Error al abrir el centro de control:', error);
	}
};

async function loadNotifications() {
	try {
		notifications.value = await getAllNotifications();
	} catch (error) {
		logError('Error loading notifications:', error);
	}
}

onMounted(async () => {
	performance.mark('panel-mounted');
	await loadNotifications();
	performance.mark('panel-ready');
	performance.measure('panel-startup', 'panel-mounted', 'panel-ready');
	// Signal backend that panel has painted - triggers deferred applets
	emit('panel-ready', {});

	// After the readiness signal: the device service is a deferred applet, so
	// it has not subscribed yet, and nothing here belongs on the path that
	// decides how fast the panel appears.
	await refreshConnectDevices();
});

// At setup, not inside onMounted: useSharedEvent registers onMounted and
// onUnmounted hooks of its own, and Vue only collects those while the component
// is being set up. Called later they never fire, and the subscription is never
// released.
useSharedEvent('connect-device-added', refreshConnectDevices);
useSharedEvent('connect-device-changed', refreshConnectDevices);
useSharedEvent('connect-device-removed', refreshConnectDevices);

// La foto entera, en una sola asignación: ver `NotificationDelta`.
//
// La campanita se sacude sólo si en la foto viene alguna notificación que antes
// no estaba. Antes bastaba con que la lista trajera algo, y como toda foto trae
// lo que quedó, la campanita se sacudía también al borrar una.
useSharedEvent<NotificationDelta>('notification-delta', (delta) => {
	const nuevas = hayNotificacionesNuevas(notifications.value, delta.items);
	notifications.value = delta.items;

	if (!nuevas) return;

	hasNewNotifications.value = true;
	clearTimeout(notificationResetTimer);
	notificationResetTimer = setTimeout(() => {
		hasNewNotifications.value = false;
	}, 1000);
});
</script>

<template>
	<nav @contextmenu.prevent="abrirMenuDelPanel" class="relative z-20 flex w-[calc(100%-8px)] justify-between items-center mx-1 h-9 mt-0.5 overflow-hidden p-1 rounded-corner bg-ui-bg/80 border border-ui-border/80 px-3">
    <div class="flex items-center gap-1">
      <img :src="menuIcon" :alt="t('views.panel.menuAlt')" @click="openMenu" class="h-7 w-7 cursor-pointer p-0.5 rounded-corner hover:bg-primary transform hover:scale-110 active:scale-95 ease-in-out" />
			<div class="w-1 h-7 bg-ui-bg/80"></div>
      <img
        :src="configIcon"
        :alt="t('views.panel.settingsAlt')"
        @click="openConfig"
        class="h-6 w-6 cursor-pointer p-0.5 rounded-corner hover:bg-primary transform hover:scale-110 active:scale-95 ease-in-out"
      />
      <img
        :src="fileManagerIcon"
        :alt="t('views.panel.filesAlt')"
        @click="openFileManager"
        class="h-6 w-6 cursor-pointer p-0.5 rounded-corner hover:bg-primary transform hover:scale-110 active:scale-95 ease-in-out"
      />
      <!-- Only while a phone is connected: a permanent button for hardware
           most people never plug in is clutter in the one strip of screen that
           is always on top of everything else. -->
      <div v-if="hasPhone" class="relative">
        <img
          :src="phoneIcon"
          :alt="t('views.connect.menuAlt')"
          :title="t('views.connect.menuAlt')"
          @click="openPhoneMenu"
          class="h-6 w-6 cursor-pointer p-0.5 rounded-corner hover:bg-primary transform hover:scale-110 active:scale-95 ease-in-out"
        />
        <div
          v-if="phoneNeedsAuth"
          :title="t('views.connect.unauthorized')"
          class="absolute -top-0.5 -right-0.5 h-2 w-2 rounded-full bg-status-warning"
        ></div>
      </div>
    </div>
    <WindowsArea />
    <div class="flex content-center items-center">
      <TrayBarArea />
      <PanelClockwidget />
      <div class="relative cursor-pointer" @click="openNotificationCenter">
        <img
          :src="notifyIcon"
          :alt="t('views.panel.notificationsAlt')"
          class="h-6 w-6 cursor-pointer p-0.5 rounded-corner hover:bg-primary transform hover:scale-110 active:scale-95 ease-in-out"
          :class="{ 'animate-bell-shake': hasNewNotifications }"
        />
        <div v-if="notifications.length > 0" class="absolute -top-0.5 -right-0.5 bg-primary text-tx-on-primary rounded-full min-w-3 h-3 flex items-center justify-center text-[8px] font-semibold leading-none px-0.5">
          {{ notifications.length > 99 ? "99+" : notifications.length }}
        </div>
      </div>
    </div>
  </nav>
</template>

