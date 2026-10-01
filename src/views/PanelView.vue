<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { Command } from '@tauri-apps/plugin-shell';
import { showContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { TrayIconButton } from '@vasakgroup/vue-libvasak';
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
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
import { reportMenuButton, toggleControlCenter, toggleMenu } from '@/services/window.service';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { useSharedEvent } from '@/tools/event.bus';
import { containsNewNotifications } from '@/tools/notifications';
import { BAR_CLASSES } from '@/tools/panel-position';
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
const { position, vertical } = usePanelConfig();

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
const openPanelContextMenu = async (event: MouseEvent) => {
	try {
		const chosen = await showContextMenu(
			[
				{
					id: 'panel',
					label: t('views.applets.panelMenu.panelSettings'),
					icon: 'preferences-system-windows',
				},
				{
					id: 'notifications',
					label: t('views.applets.panelMenu.notifications'),
					icon: 'preferences-desktop-notification',
				},
				{ type: 'separator' },
				{
					id: 'system',
					label: t('views.applets.panelMenu.systemSettings'),
					icon: 'preferences-system',
				},
			],
			event,
			{ window: true }
		);

		switch (chosen?.id) {
			case 'panel':
				await invoke('open_settings_section', { section: 'appearance-panel' });
				break;
			case 'notifications':
				await toggleControlCenter();
				break;
			case 'system':
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

/**
 * El botón del menú: de él cuelga el menú, lo abra un clic o la tecla Super.
 */
/**
 * El botón del menú. Es un componente de la librería, así que el `ref` es la
 * instancia: `anchorOf` sabe leerle el `$el`, y el observador necesita el
 * elemento.
 */
const menuButton = ref<{ $el?: Element } | null>(null);
const { isOpen: menuIsOpen, openClasses: menuOpenClasses } = useOpenApplet('menu');

const openMenu = async () => {
	try {
		await toggleMenu(menuButton.value);
	} catch (error) {
		logError('Error al abrir el menu:', error);
	}
};

/**
 * Le cuenta al backend dónde quedó el botón del menú, para que abrirlo sin clic
 * —la tecla Super, por D-Bus— lo cuelgue de acá y no del centro.
 *
 * Al montarse, cuando el panel cambia de lado y cuando el botón cambia de
 * tamaño. Con `ResizeObserver` y no con `resize`: en este WebView `resize` no
 * llega.
 */
const sendMenuButton = async () => {
	try {
		await reportMenuButton(position.value, menuButton.value);
	} catch (error) {
		logError('No se pudo informar el botón del menú:', error);
	}
};

let menuButtonObserver: ResizeObserver | undefined;

watch(position, async () => {
	await nextTick();
	await sendMenuButton();
});

onMounted(() => {
	void sendMenuButton();
	const element = menuButton.value?.$el;
	if (typeof ResizeObserver !== 'undefined' && element instanceof Element) {
		menuButtonObserver = new ResizeObserver(() => void sendMenuButton());
		menuButtonObserver.observe(element);
	}
});

onBeforeUnmount(() => menuButtonObserver?.disconnect());

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
	const hasNew = containsNewNotifications(notifications.value, delta.items);
	notifications.value = delta.items;

	if (!hasNew) return;

	hasNewNotifications.value = true;
	clearTimeout(notificationResetTimer);
	notificationResetTimer = setTimeout(() => {
		hasNewNotifications.value = false;
	}, 1000);
});
</script>

<template>
	<!-- El panel flota sobre el escritorio: la superficie opaca `ui-float`, el
	     canto fino y el radio `l`, que es el de un contenedor con `p-1`
	     alrededor de botones `m` (el anidado de Once UI). Sus botones son los
	     de la bandeja de la librería (`TrayIconButton`): el velo neutro al
	     pasar en lugar del relleno del primario, sin escala, y el anillo de
	     foco por dentro, que en una barra de 36 píxeles no se corta. -->
	<nav
		@contextmenu.prevent="openPanelContextMenu"
		class="relative z-20 flex justify-between items-center overflow-hidden p-1 rounded-corner-l bg-ui-float border border-ui-line"
		:class="BAR_CLASSES[position]"
	>
    <div class="flex items-center gap-1" :class="vertical ? 'flex-col' : ''">
      <TrayIconButton
        ref="menuButton"
        name="start-here"
        :alt="t('views.panel.menuAlt')"
        :tooltip="t('views.panel.menuAlt')"
        :custom-class="menuOpenClasses"
        :aria-expanded="menuIsOpen"
        @click="openMenu"
      />
			<!-- El separador gira con la barra: de costado, una raya vertical de
			     un píxel de ancho entre dos iconos apilados no separa nada. -->
			<div class="bg-ui-line" :class="vertical ? 'h-px w-7' : 'w-px h-7'"></div>
      <TrayIconButton
        name="preferences-system"
        :alt="t('views.panel.settingsAlt')"
        :tooltip="t('views.panel.settingsAlt')"
        @click="openConfig"
      />
      <TrayIconButton
        name="system-file-manager"
        :alt="t('views.panel.filesAlt')"
        :tooltip="t('views.panel.filesAlt')"
        @click="openFileManager"
      />
      <!-- Only while a phone is connected: a permanent button for hardware
           most people never plug in is clutter in the one strip of screen that
           is always on top of everything else. -->
      <TrayIconButton
        v-if="hasPhone"
        name="smartphone"
        :alt="t('views.connect.menuAlt')"
        :tooltip="phoneNeedsAuth ? t('views.connect.unauthorized') : t('views.connect.menuAlt')"
        @click="openPhoneMenu"
      >
        <div
          v-if="phoneNeedsAuth"
          class="absolute -top-0.5 -right-0.5 h-2 w-2 rounded-corner-full bg-status-warning"
        ></div>
      </TrayIconButton>
    </div>
    <WindowsArea />
    <div class="flex content-center items-center" :class="vertical ? 'flex-col' : ''">
      <TrayBarArea />
      <PanelClockwidget />
      <!-- La insignia la dibuja el botón de la librería; el número no pasa de
           99 para que entre en la píldora. -->
      <TrayIconButton
        name="preferences-desktop-notification"
        :alt="t('views.panel.notificationsAlt')"
        :tooltip="t('views.panel.notificationsAlt')"
        :badge="notifications.length > 0 ? Math.min(notifications.length, 99) : null"
        :icon-class="{ 'animate-bell-shake': hasNewNotifications }"
        @click="openNotificationCenter"
      />
    </div>
  </nav>
</template>

