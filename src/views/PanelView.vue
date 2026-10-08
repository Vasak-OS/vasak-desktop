<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { showContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { Badge, PanelPill, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import TrayBarArea from '@/components/areas/panel/TrayBarArea.vue';
import WindowsArea from '@/components/areas/panel/WindowsArea.vue';
import TrayIconBattery from '@/components/buttons/TrayIconBattery.vue';
import TrayIconBluetooth from '@/components/buttons/TrayIconBluetooth.vue';
import TrayIconNetwork from '@/components/buttons/TrayIconNetwork.vue';
import TrayIconSound from '@/components/buttons/TrayIconSound.vue';
import TrayMusicControl from '@/components/controls/TrayMusicControl.vue';
import TrayWeatherControl from '@/components/controls/TrayWeatherControl.vue';
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
	showWeather,
	showMusic,
	barClasses,
	surfaceClass,
	hasSurface,
	animationClass,
	sizeStyle,
	autohide,
} = usePanelConfig();

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

/** La clase que desliza la barra fuera de la pantalla mientras está escondida. */
const hideClass = computed(() => panelHideClass(position.value, panelHidden.value));

/**
 * Cuánto texto entra: en un panel angosto los nombres largos se pliegan al
 * icono (`panel-density.ts`), medido sobre la propia barra.
 */
watchPanelDensity(bar, vertical);
const density = usePanelDensity();
const windowsFirst = computed(() => density.value === 'tight');

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
 * El botón del menú, el lince: de él cuelga el menú, lo abra un clic o la
 * tecla Super. Es una píldora de la librería, así que el `ref` es la
 * instancia: `anchorOf` sabe leerle el `$el`, y el observador necesita el
 * elemento.
 */
const menuButton = ref<{ $el?: Element } | null>(null);
const { isOpen: menuIsOpen } = useOpenApplet('menu');

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
	<!-- El panel en píldoras flotantes (vasak-desktop#151), como el video de
	     referencia: la barra ya no es una franja continua sino píldoras sueltas
	     sobre el escritorio. La `<nav>` es transparente y sólo reparte: a la
	     izquierda el menú, los accesos fijos, las notificaciones, los espacios
	     de trabajo, la música y las ventanas; al centro el reloj y el clima; a la derecha la
	     bandeja, el teclado, la red, el Bluetooth, el volumen y la batería.
	     Cada píldora es una `PanelPill` de la librería en `ui-shell`, sin
	     `backdrop-blur`: el desenfoque lo pone Wayfire detrás de cada una.

	     El tipo, la densidad, la animación y el tamaño salen de la configuración
	     (`panel-appearance.ts`): en píldoras la `<nav>` queda transparente y la
	     región de entrada se recorta a las píldoras; en flotante, barra y dock se
	     dibuja una superficie detrás y la región pasa a ser la barra entera
	     (`usePanelInputRegion`). -->
	<nav
		ref="bar"
		@contextmenu.prevent="openPanelContextMenu"
		class="panel-bar z-20 grid items-center gap-2 bg-transparent transition-transform duration-200"
		:class="[barClasses, animationClass, hideClass]"
		:style="sizeStyle"
		data-panel-bar
	>
    <!-- La superficie continua de flotante, barra y dock, detrás de las
         píldoras. En píldoras no existe: la `<nav>` es transparente y entre una
         píldora y otra se ve el escritorio. Es translúcida y sin `backdrop-blur`:
         el desenfoque lo pone Wayfire detrás de la franja. Su forma —el fondo, el
         canto y el redondeo— sale de `panelSurfaceClass`. -->
    <div
      v-if="hasSurface"
      aria-hidden="true"
      class="pointer-events-none absolute inset-0 -z-10"
      :class="surfaceClass"
      data-panel-surface
    ></div>
    <div class="flex min-w-0 items-center gap-1.5 overflow-x-clip" :class="GROUP_CLASSES[position].start" data-panel-start>
      <!-- El botón del menú: el lince de VasakOS (`start-here` del tema), como
           siempre. Abre el menú con la búsqueda enfocada, lo abra este botón o
           la tecla Super (tests/menu-search-focus.test.ts). -->
      <PanelPill
        ref="menuButton"
        icon="start-here"
        icon-type="icon"
        :accessible-label="t('views.panel.menuAlt')"
        :title="t('views.panel.menuAlt')"
        :expanded="menuIsOpen"
        :orientation="vertical ? 'vertical' : 'horizontal'"
        class="shrink-0"
        data-menu-pill
        @click="openMenu"
      />
      <!-- Configuración y Archivos: la zona de los accesos fijos, donde después
           van las aplicaciones ancladas. -->
      <PinnedAppsPill />
      <!-- En el panel más angosto las ventanas suben al lado de los accesos
           fijos: así, si lo de la izquierda no entra, lo que se recorta es la
           música o los espacios de trabajo y no la barra de ventanas, que no
           sale nunca (decisión del usuario, 03/10/2026). -->
      <WindowsArea v-if="windowsFirst" />
      <!-- Only while a phone is connected: a permanent button for hardware
           most people never plug in is clutter in the one strip of screen that
           is always on top of everything else. -->
      <PanelPill
        v-if="hasPhone"
        icon="smartphone"
        icon-type="icon"
        :accessible-label="phoneNeedsAuth ? t('views.connect.unauthorized') : t('views.connect.menuAlt')"
        :title="phoneNeedsAuth ? t('views.connect.unauthorized') : t('views.connect.menuAlt')"
        :orientation="vertical ? 'vertical' : 'horizontal'"
        class="shrink-0"
        data-phone-pill
        @click="openPhoneMenu"
      >
        <span
          v-if="phoneNeedsAuth"
          class="absolute top-0.5 right-0.5 size-2 rounded-corner-full bg-status-warning"
        ></span>
      </PanelPill>
      <WorkspacesPill />
      <TrayMusicControl v-if="showMusic" />
      <WindowsArea v-if="!windowsFirst" />
    </div>
    <div class="flex items-center gap-1.5" :class="GROUP_CLASSES[position].center" data-panel-center>
      <PanelClockWidget />
      <TrayWeatherControl v-if="showWeather" />
    </div>
    <div class="flex min-w-0 items-center gap-1.5 overflow-x-clip" :class="GROUP_CLASSES[position].end" data-panel-end>
      <TrayBarArea />
      <KeyboardLayoutPill />
      <TrayIconNetwork />
      <TrayIconBluetooth />
      <TrayIconSound />
      <TrayIconBattery />
      <!-- La campanita vuelve al extremo del panel, junto a la bandeja, como
           antes del pasaje a píldoras (vasak-desktop#160): es el último grupo,
           así que queda en el final de la barra sin importar de qué lado esté.
           El número no pasa de 99 para que entre en la píldora. -->
      <PanelPill
        :accessible-label="t('views.panel.notificationsAlt')"
        :title="t('views.panel.notificationsAlt')"
        :orientation="vertical ? 'vertical' : 'horizontal'"
        class="shrink-0"
        data-notifications-pill
        @click="openNotificationCenter"
      >
        <template #leading>
          <ThemeIcon
            name="preferences-desktop-notification"
            type="symbol"
            :size="18"
            alt=""
            class="shrink-0"
            :class="{ 'animate-bell-shake': hasNewNotifications }"
          />
        </template>
        <Badge
          v-if="notifications.length > 0 && !vertical"
          tone="accent"
          variant="solid"
          counter
          :label="notifications.length"
          :max="99"
        />
      </PanelPill>
    </div>
  </nav>
</template>
