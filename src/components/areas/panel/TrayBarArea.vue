<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { isBluetoothPluginInitialized } from '@vasakgroup/plugin-bluetooth-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ProgressBar } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, type Ref, ref } from 'vue';
import TrayIconBattery from '@/components/buttons/TrayIconBattery.vue';
import TrayIconBluetooth from '@/components/buttons/TrayIconBluetooth.vue';
import TrayIconCapsLock from '@/components/buttons/TrayIconCapsLock.vue';
import TrayIconMicrophone from '@/components/buttons/TrayIconMicrophone.vue';
import TrayIconNetwork from '@/components/buttons/TrayIconNetwork.vue';
import TrayIconPrivacy from '@/components/buttons/TrayIconPrivacy.vue';
import TrayIconSound from '@/components/buttons/TrayIconSound.vue';
import TrayIconTwingate from '@/components/buttons/TrayIconTwingate.vue';
import TrayItemButton from '@/components/buttons/TrayItemButton.vue';
import TrayMusicControl from '@/components/controls/TrayMusicControl.vue';
import TrayNetworkRateControl from '@/components/controls/TrayNetworkRateControl.vue';
import TrayWeatherControl from '@/components/controls/TrayWeatherControl.vue';
import Badge from '@/components/indicators/Badge.vue';
import type { TrayItem } from '@/interfaces/tray';
import { batteryExists } from '@/services/core.service';
import {
	getTrayItems,
	initSniWatcher,
	openTrayPopup,
	trayItemActivate,
	trayItemSecondaryActivate,
} from '@/services/tray.service';
import { animationBudget } from '@/tools/animation.budget';
import { anchorOf } from '@/tools/applet-anchor';
import { OPEN_APPLET_CLASSES, useOpenApplet } from '@/tools/composables/useOpenApplet';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { useSharedEvent } from '@/tools/event.bus';
import { createSerialQueue } from '@/tools/serial-queue';
import {
	countLabel,
	itemName,
	needsAttention,
	progressPercent,
	tooltipText,
} from '@/tools/tray-item';
import { logError, logWarning } from '@/utils/logger';

// Qué partes del panel están encendidas, y de qué lado va la barra: a los
// costados los iconos se apilan en lugar de alinearse.
const { showWeather, showMusic, showTransfer, showTray, showPrivacy, vertical } = usePanelConfig();

const { t } = useI18n();

const bluetoothInitialized: Ref<boolean> = ref(false);
const existBattery: Ref<boolean> = ref(false);
const trayItems = ref<TrayItem[]>([]);

/**
 * When more than 8 tray items are present, disable per-item entrance
 * animations to avoid frame drops. Items render in a single paint instead.
 * @requirements 15.3
 */
const shouldAnimate = computed(() => trayItems.value.length <= 8);

/** Animation event handlers wired to AnimationBudgetManager */
const onAnimationStart = (event: AnimationEvent) => {
	const el = event.target as HTMLElement;
	animationBudget.manageWillChange(el, true);
};

const onAnimationEnd = (event: AnimationEvent) => {
	const el = event.target as HTMLElement;
	animationBudget.manageWillChange(el, false);
	animationBudget.releaseSlot();
};

const onTransitionStart = (event: TransitionEvent) => {
	const el = event.target as HTMLElement;
	animationBudget.manageWillChange(el, true);
};

const onTransitionEnd = (event: TransitionEvent) => {
	const el = event.target as HTMLElement;
	animationBudget.manageWillChange(el, false);
	animationBudget.releaseSlot();
};

const refreshTrayItems = async (): Promise<void> => {
	try {
		trayItems.value = await getTrayItems();
	} catch (error) {
		logError('[TrayPanel] Error obteniendo items del tray:', error);
	}
};

/**
 * De qué icono es el menú abierto.
 *
 * El applet de la bandeja es uno solo para todos los iconos: el backend dice
 * que está abierto, y esto dice cuál de los iconos lo abrió, para realzar ése.
 */
const trayPopupOwner = ref<string | null>(null);
const { isOpen: trayPopupOpen } = useOpenApplet('tray');

/** Un menú por vez: ver `serial-queue.ts`. */
const trayPopupQueue = createSerialQueue();

const isTrayPopupOwner = (item: TrayItem) =>
	trayPopupOpen.value && trayPopupOwner.value === item.service_name;

/**
 * `ItemIsMenu`: el elemento sólo tiene menú, y la especificación pide abrirlo
 * también con el clic principal en vez de mandarle `Activate`.
 */
const opensMenu = (item: TrayItem, event: MouseEvent) =>
	event.button === 2 || (event.button === 0 && item.item_is_menu === true);

/** La barra de progreso nombra a quién progresa, para un lector de pantalla. */
const progressLabel = (item: TrayItem) =>
	t('components.tray.progress').replace('{0}', itemName(item));

const handleTrayClick = async (item: TrayItem, event: MouseEvent) => {
	try {
		if (opensMenu(item, event)) {
			event.preventDefault();
			trayPopupOwner.value = item.service_name;
			// El rectángulo se mide ahora: `currentTarget` sólo vale mientras
			// dura el evento, y el pedido puede esperar en la fila.
			const anchor = anchorOf(event.currentTarget) ?? null;
			await trayPopupQueue(() => openTrayPopup({ serviceName: item.service_name, anchor }));
		} else if (event.button === 0) {
			await trayItemActivate({
				serviceName: item.service_name,
				x: event.clientX,
				y: event.clientY,
			});
		} else if (event.button === 1) {
			await trayItemSecondaryActivate({
				serviceName: item.service_name,
				x: event.clientX,
				y: event.clientY,
			});
		}
	} catch (error) {
		logError('[TrayPanel] Error manejando click:', error);
	}
};

const getItemPulseClass = (item: TrayItem) => {
	return needsAttention(item) ? 'animate-[pulse-attention_2s_infinite_ease-in-out]' : '';
};

const getItemStatusClass = (item: TrayItem) => {
	switch (item.status) {
		case 'Active':
			return 'tray-item-active';
		case 'Passive':
			return 'tray-item-passive';
		case 'NeedsAttention':
			return 'tray-item-attention';
		default:
			return '';
	}
};

onMounted(async () => {
	await refreshTrayItems();
	bluetoothInitialized.value = await isBluetoothPluginInitialized();
	try {
		existBattery.value = await batteryExists();
	} catch (e) {
		logWarning('[TrayPanel] batteryExists failed:', e);
		existBattery.value = false;
	}
	try {
		await initSniWatcher();
	} catch (error) {
		logWarning('[TryPanel] Init SNI Watcher (already running or unavailable)', error);
	}
});

useSharedEvent('tray-update', refreshTrayItems);

useSharedEvent<{ has_battery?: boolean }>('battery-update', (payload) => {
	if (typeof payload?.has_battery === 'boolean') {
		existBattery.value = payload.has_battery;
	}
});
</script>

<template>
  <div
    class="flex items-center gap-1"
    :class="vertical ? 'flex-col py-2 w-full' : 'px-2 h-full'"
  >
    <TransitionGroup
      :move-class="shouldAnimate ? 'transition-transform duration-300 ease-[cubic-bezier(0.25,0.8,0.25,1)]' : ''"
      :enter-active-class="shouldAnimate ? 'transition-all duration-300 ease-[cubic-bezier(0.25,0.8,0.25,1)]' : ''"
      :leave-active-class="shouldAnimate ? 'transition-all duration-300 ease-[cubic-bezier(0.55,0,0.45,1)]' : ''"
      :enter-from-class="shouldAnimate ? 'opacity-0 -translate-x-5 scale-80 -rotate-12' : ''"
      :leave-to-class="shouldAnimate ? 'opacity-0 translate-x-5 scale-80 rotate-12' : ''"
      tag="div"
      class="flex items-center gap-1"
      :class="vertical ? 'flex-col' : ''"
    >
      <TrayNetworkRateControl v-if="showTransfer" key="network-rate" />
      <TrayWeatherControl v-if="showWeather" key="weather" />
      <TrayMusicControl v-if="showMusic" key="music-control" />
      <div
        v-for="item in (showTray ? trayItems : [])"
        :key="item.service_name"
        :class="[
          'relative flex items-center justify-center w-7 h-7 rounded-corner-m cursor-pointer transition-colors duration-200 ease-ui hover:bg-ui-hover active:bg-ui-pressed group',
          getItemStatusClass(item),
          getItemPulseClass(item),
          { [OPEN_APPLET_CLASSES]: isTrayPopupOwner(item) },
        ]"
        @mousedown.prevent="(e) => handleTrayClick(item, e)"
        @contextmenu.prevent.stop
        @animationstart="onAnimationStart"
        @animationend="onAnimationEnd"
        @transitionstart="onTransitionStart"
        @transitionend="onTransitionEnd"
        :title="tooltipText(item)"
      >
        <!-- Icono: el mapa de bits propio, el tema o la inicial; y la insignia
             superpuesta si la manda -->
        <TrayItemButton :item="item" />

        <!-- El contador de LauncherEntry, sólo si la aplicación lo hace visible.
             Copia provisoria de la Badge de la 2.1.0: ver Badge.vue. -->
        <Badge
          v-if="countLabel(item.launcher?.count)"
          tone="accent"
          variant="solid"
          class="pointer-events-none absolute -top-1 -right-1"
          :label="countLabel(item.launcher?.count)"
        />
        <!-- Pide atención y no hay contador que ya lo diga: el punto. -->
        <div
          v-else-if="needsAttention(item)"
          data-tray-attention
          class="absolute -top-1 -right-1 size-2 rounded-corner-full bg-status-error animate-pulse"
        />

        <!-- El progreso de LauncherEntry, sólo con progress-visible. -->
        <div
          v-if="progressPercent(item.launcher?.progress) !== undefined"
          data-tray-progress
          class="pointer-events-none absolute inset-x-0.5 -bottom-1"
        >
          <ProgressBar
            :value="progressPercent(item.launcher?.progress) ?? 0"
            :label="progressLabel(item)"
          />
        </div>
      </div>
      <TrayIconPrivacy v-if="showPrivacy" key="icon-privacy" />
      <TrayIconSound key="icon-sound" />
      <TrayIconBattery v-if="existBattery" key="icon-battery" />
      <TrayIconCapsLock key="icon-capslock" />
      <TrayIconMicrophone key="icon-micmute" />
      <TrayIconBluetooth key="icon-bluetooth" />
      <TrayIconTwingate key="icon-twingate" />
      <TrayIconNetwork key="icon-network" />
    </TransitionGroup>
  </div>
</template>

