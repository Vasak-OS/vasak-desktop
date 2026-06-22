<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { isBluetoothPluginInitialized } from '@vasakgroup/plugin-bluetooth-manager';
import { onMounted, type Ref, ref } from 'vue';
import TrayIconBattery from '@/components/buttons/TrayIconBattery.vue';
import TrayIconBluetooth from '@/components/buttons/TrayIconBluetooth.vue';
import TrayIconNetwork from '@/components/buttons/TrayIconNetwork.vue';
import TrayIconSound from '@/components/buttons/TrayIconSound.vue';
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
import { useEventListener } from '@/tools/event.listener';
import { logError, logWarning } from '@/utils/logger';

const bluetoothInitialized: Ref<boolean> = ref(false);
const existBattery: Ref<boolean> = ref(false);
const trayItems = ref<TrayItem[]>([]);

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

useEventListener('tray-update', refreshTrayItems);

useEventListener('battery-update', (event) => {
	const payload: any = event.payload || {};
	if (typeof payload.has_battery === 'boolean') {
		existBattery.value = payload.has_battery;
	}
});
</script>

<template>
  <div class="flex items-center gap-1 px-2 h-full">
    <TransitionGroup move-class="transition-transform duration-400 ease-[cubic-bezier(0.25,0.8,0.25,1)]" enter-active-class="transition-all duration-400 ease-[cubic-bezier(0.25,0.8,0.25,1)]" leave-active-class="transition-all duration-300 ease-[cubic-bezier(0.55,0,0.45,1)]" enter-from-class="opacity-0 -translate-x-5 scale-80 -rotate-12" leave-to-class="opacity-0 translate-x-5 scale-80 rotate-12" tag="div" class="flex items-center gap-1">
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
        :title="item.tooltip || item.title"
      >
        <!-- Icon with loading state -->
        <div class="relative w-4 h-4 flex items-center justify-center">
          <img
            v-if="item.icon_data"
            :src="`data:image/png;base64,${item.icon_data}`"
            :alt="item.title || item.service_name"
            class="w-4 h-4 object-contain transition-all duration-300 group-hover:brightness-110 group-hover:scale-110 drop-shadow-[0_1px_2px_rgba(0,0,0,0.3)]"
            @error="($event.target as HTMLImageElement).style.display = 'none'"
          />
          <div
            v-else
            class="w-4 h-4 object-contain transition-all duration-300 group-hover:brightness-110 group-hover:scale-110 drop-shadow-[0_1px_2px_rgba(0,0,0,0.3)]-placeholder"
            :class="{ 'animate-pulse': !item.icon_data }"
          />
        </div>

        <!-- Status indicator -->
        <div v-if="item.status === 'NeedsAttention'" class="absolute -top-1 -right-1 w-2 h-2 bg-red-500 rounded-full animate-pulse shadow-lg shadow-red-500/50" />
      </div>
      <TrayIconSound key="icon-sound" />
      <TrayIconBattery v-if="existBattery" key="icon-battery" />
      <TrayIconBluetooth key="icon-bluetooth" />
      <TrayIconNetwork key="icon-network" />
    </TransitionGroup>
  </div>
</template>

