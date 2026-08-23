<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { isBluetoothPluginInitialized } from '@vasakgroup/plugin-bluetooth-manager';
import { computed, onMounted, type Ref, ref } from 'vue';
import TrayIconBattery from '@/components/buttons/TrayIconBattery.vue';
import TrayIconBluetooth from '@/components/buttons/TrayIconBluetooth.vue';
import TrayIconCapsLock from '@/components/buttons/TrayIconCapsLock.vue';
import TrayIconMicrophone from '@/components/buttons/TrayIconMicrophone.vue';
import TrayIconNetwork from '@/components/buttons/TrayIconNetwork.vue';
import TrayIconSound from '@/components/buttons/TrayIconSound.vue';
import TrayIconTwingate from '@/components/buttons/TrayIconTwingate.vue';
import TrayItemButton from '@/components/buttons/TrayItemButton.vue';
import TrayMusicControl from '@/components/controls/TrayMusicControl.vue';
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
import { useSharedEvent } from '@/tools/event.bus';
import { logError, logWarning } from '@/utils/logger';

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

const handleTrayClick = async (item: TrayItem, event: MouseEvent) => {
	console.log('[TrayPanel] handleTrayClick', event.button, item.service_name, item.menu_path);
	try {
		if (event.button === 2) {
			event.preventDefault();
			await openTrayPopup({ serviceName: item.service_name });
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
	return item.status === 'NeedsAttention'
		? 'animate-[pulse-attention_2s_infinite_ease-in-out]'
		: '';
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
  <div class="flex items-center gap-1 px-2 h-full">
    <TransitionGroup
      :move-class="shouldAnimate ? 'transition-transform duration-400 ease-[cubic-bezier(0.25,0.8,0.25,1)]' : ''"
      :enter-active-class="shouldAnimate ? 'transition-all duration-400 ease-[cubic-bezier(0.25,0.8,0.25,1)]' : ''"
      :leave-active-class="shouldAnimate ? 'transition-all duration-300 ease-[cubic-bezier(0.55,0,0.45,1)]' : ''"
      :enter-from-class="shouldAnimate ? 'opacity-0 -translate-x-5 scale-80 -rotate-12' : ''"
      :leave-to-class="shouldAnimate ? 'opacity-0 translate-x-5 scale-80 rotate-12' : ''"
      tag="div"
      class="flex items-center gap-1"
    >
      <TrayMusicControl key="music-control" />
      <div
        v-for="item in trayItems"
        :key="item.service_name"
        :class="[
          'relative flex items-center justify-center w-7 h-7 rounded-corner cursor-pointer transform transition-all duration-300 ease-out hover:bg-white/15 hover:scale-110 hover:rotate-3 active:scale-95 active:rotate-0 group',
          getItemStatusClass(item),
          getItemPulseClass(item),
        ]"
        @mousedown.prevent="(e) => handleTrayClick(item, e)"
        @contextmenu.prevent
        @animationstart="onAnimationStart"
        @animationend="onAnimationEnd"
        @transitionstart="onTransitionStart"
        @transitionend="onTransitionEnd"
        :title="item.tooltip || item.title"
      >
        <!-- Icon: own pixmap, then the theme, then the initial -->
        <div class="relative w-4 h-4 flex items-center justify-center">
          <TrayItemButton :item="item" />
        </div>

        <!-- Status indicator -->
        <div v-if="item.status === 'NeedsAttention'" class="absolute -top-1 -right-1 w-2 h-2 bg-red-500 rounded-full animate-pulse shadow-lg shadow-red-500/50" />
      </div>
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

