<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { isBluetoothPluginInitialized } from '@vasakgroup/plugin-bluetooth-manager';
import { onMounted, onUnmounted, type Ref, ref } from 'vue';
import TrayIconBattery from '@/components/buttons/TrayIconBattery.vue';
import TrayIconBluetooth from '@/components/buttons/TrayIconBluetooth.vue';
import TrayIconNetwork from '@/components/buttons/TrayIconNetwork.vue';
import TrayIconSound from '@/components/buttons/TrayIconSound.vue';
import TrayMusicControl from '@/components/controls/TrayMusicControl.vue';
import type { TrayItem, TrayMenu } from '@/interfaces/tray';
import {
	getTrayMenu,
	trayItemActivate,
	trayItemSecondaryActivate,
	trayMenuItemClick,
} from '@/services/tray.service';
import { batteryExists } from '@/services/core.service';
import { getTrayItems, initSniWatcher } from '@/services/tray.service';
import { logError, logWarning } from '@/utils/logger';

const bluetoothInitialized: Ref<boolean> = ref(false);
const existBattery: Ref<boolean> = ref(false);
const trayItems = ref<TrayItem[]>([]);
const contextMenu = ref<{
	visible: boolean;
	x: number;
	y: number;
	items: TrayMenu[];
	trayId: string;
}>({
	visible: false,
	x: 0,
	y: 0,
	items: [],
	trayId: '',
});

let unlisten: (() => void) | null = null;
let unlistenBatteryEvent: (() => void) | null = null;

const refreshTrayItems = async (): Promise<void> => {
	try {
		trayItems.value = await getTrayItems();
	} catch (error) {
		logError('[TrayPanel] Error obteniendo items del tray:', error);
	}
};

const handleTrayClick = async (item: TrayItem, event: MouseEvent) => {
	try {
		if (event.button === 2) {
			// Right click
			event.preventDefault();
			await showContextMenu(item, event);
		} else if (event.button === 0) {
			// Left click
			await trayItemActivate({
				service_name: item.service_name,
				x: event.clientX,
				y: event.clientY,
			});
		} else if (event.button === 1) {
			// Middle click
			await trayItemSecondaryActivate({
				service_name: item.service_name,
				x: event.clientX,
				y: event.clientY,
			});
		}
	} catch (error) {
		logError('[TrayPanel] Error manejando click:', error);
	}
};

const showContextMenu = async (item: TrayItem, event: MouseEvent) => {
	if (!item.menu_path) return;

	try {
		const menuItems: TrayMenu[] = await getTrayMenu({
			service_name: item.service_name,
		});

		contextMenu.value = {
			visible: true,
			x: event.clientX,
			y: event.clientY,
			items: menuItems,
			trayId: item.service_name,
		};
	} catch (error) {
		logError('[TrayPanel] Error obteniendo menú:', error);
	}
};

const handleMenuItemClick = async (menuItem: TrayMenu) => {
	try {
		await trayMenuItemClick({
			service_name: contextMenu.value.trayId,
			menu_id: menuItem.id,
		});
		contextMenu.value.visible = false;
	} catch (error) {
		logError('[TrayPanel] Error en click de menú:', error);
	}
};

const hideContextMenu = () => {
	contextMenu.value.visible = false;
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
	unlisten = await listen('tray-update', refreshTrayItems);
	bluetoothInitialized.value = await isBluetoothPluginInitialized();
	// Inicial: consultar si existe batería
	try {
		existBattery.value = await batteryExists();
	} catch (e) {
		logWarning('[TrayPanel] batteryExists failed:', e);
		existBattery.value = false;
	}

	// Suscribirse a eventos de batería para actualizar visibilidad en caliente
	unlistenBatteryEvent = await listen('battery-update', (event) => {
		const payload: any = event.payload || {};
		if (typeof payload.has_battery === 'boolean') {
			existBattery.value = payload.has_battery;
		}
	});
	try{
		await initSniWatcher
	} catch (error){
		logError('[TryPanel] Init SNI Watcher', error)
	}

	document.addEventListener('click', hideContextMenu);
});

onUnmounted(() => {
	unlisten?.();
	unlistenBatteryEvent?.();
	document.removeEventListener('click', hideContextMenu);
});
</script>

<template>
  <div class="flex items-center gap-1 px-2 h-full">
    <TransitionGroup move-class="transition-transform duration-400 ease-[cubic-bezier(0.25,0.8,0.25,1)]" enter-active-class="transition-all duration-400 ease-[cubic-bezier(0.25,0.8,0.25,1)]" leave-active-class="transition-all duration-300 ease-[cubic-bezier(0.55,0,0.45,1)]" enter-from-class="opacity-0 -translate-x-5 scale-80 -rotate-12" leave-to-class="opacity-0 translate-x-5 scale-80 rotate-12" tag="div" class="flex items-center gap-1">
      <TrayMusicControl />
      <div
        v-for="item in trayItems"
        :key="item.service_name"
        :class="[
          'relative flex items-center justify-center w-7 h-7 rounded-corner cursor-pointer transform transition-all duration-300 ease-out hover:bg-white/15 hover:scale-110 hover:rotate-3 active:scale-95 active:rotate-0 group',
          getItemStatusClass(item),
          getItemPulseClass(item),
        ]"
        @click="(e) => handleTrayClick(item, e)"
        @contextmenu.prevent="(e) => handleTrayClick(item, e)"
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
      <TrayIconSound />
      <TrayIconBattery v-if="existBattery" />
      <TrayIconBluetooth />
      <TrayIconNetwork />
    </TransitionGroup>

    <!-- Context Menu -->
    <Teleport to="body">
      <Transition enter-active-class="transition-all duration-200 ease-[cubic-bezier(0.25,0.8,0.25,1)]" leave-active-class="transition-all duration-150 ease-[cubic-bezier(0.55,0,0.45,1)]" enter-from-class="opacity-0 -translate-y-full scale-95" leave-to-class="opacity-0 -translate-y-full scale-95">
        <div
          v-if="contextMenu.visible"
          class="fixed z-50 bg-white/95 dark:bg-black/95 backdrop-blur-md border border-gray-200/50 dark:border-gray-700/50 rounded-corner shadow-2xl py-2 min-w-48 max-w-64"
          :style="{
            left: `${contextMenu.x}px`,
            top: `${contextMenu.y - 10}px`,
            transform: 'translateY(-100%)',
          }"
          @click.stop
        >
          <div
            v-for="(menuItem) in contextMenu.items"
            :key="menuItem.id"
            :class="[
              'flex items-center justify-between px-4 py-2 text-sm cursor-pointer transition-colors duration-200 hover:bg-gray-100/50 dark:hover:bg-gray-800/50',
              {
                disabled: !menuItem.enabled,
                separator: menuItem.type === 'separator',
                checked: menuItem.checked,
              },
            ]"
            @click="menuItem.enabled && handleMenuItemClick(menuItem)"
          >
            <span v-if="menuItem.type !== 'separator'" class="flex-1 text-left">
              {{ menuItem.label }}
            </span>
            <div v-if="menuItem.checked" class="text-blue-600 dark:text-blue-400 font-bold ml-2">✓</div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

