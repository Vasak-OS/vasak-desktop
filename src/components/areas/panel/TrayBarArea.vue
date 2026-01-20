<script setup lang="ts">
import { ref, onMounted, onUnmounted, Ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import TrayIconBluetooth from '@/components/buttons/TrayIconBluetooth.vue';
import { isBluetoothPluginInitialized } from '@vasakgroup/plugin-bluetooth-manager';
import TrayIconNetwork from '@/components/buttons/TrayIconNetwork.vue';
import TrayIconSound from '@/components/buttons/TrayIconSound.vue';
import TrayMusicControl from '@/components/controls/TrayMusicControl.vue';
import TrayIconBattery from '@/components/buttons/TrayIconBattery.vue';
import { TrayItem, TrayMenu } from '@/interfaces/tray';
import { batteryExists } from '@/tools/battery.controller';
import { getTrayItems, startSNIWatcher } from '@/tools/tray.controller';

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
		console.error('[TrayPanel Error] Error obteniendo items del tray:', error);
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
			await invoke('tray_item_activate', {
				service_name: item.service_name,
				x: event.clientX,
				y: event.clientY,
			});
		} else if (event.button === 1) {
			// Middle click
			await invoke('tray_item_secondary_activate', {
				service_name: item.service_name,
				x: event.clientX,
				y: event.clientY,
			});
		}
	} catch (error) {
		console.error('[TrayPanel Error] Error manejando click:', error);
	}
};

const showContextMenu = async (item: TrayItem, event: MouseEvent) => {
	if (!item.menu_path) return;

	try {
		const menuItems: TrayMenu[] = await invoke('get_tray_menu', {
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
		console.error('[TrayPanel Error] Error obteniendo menú:', error);
	}
};

const handleMenuItemClick = async (menuItem: TrayMenu) => {
	try {
		await invoke('tray_menu_item_click', {
			service_name: contextMenu.value.trayId,
			menu_id: menuItem.id,
		});
		contextMenu.value.visible = false;
	} catch (error) {
		console.error('[TrayPanel Error] Error en click de menú:', error);
	}
};

const hideContextMenu = () => {
	contextMenu.value.visible = false;
};

const getItemPulseClass = (item: TrayItem) => {
	return item.status === 'NeedsAttention' ? 'animate-pulse-attention' : '';
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
		console.warn('[TrayPanel] batteryExists failed:', e);
		existBattery.value = false;
	}

	// Suscribirse a eventos de batería para actualizar visibilidad en caliente
	unlistenBatteryEvent = await listen('battery-update', (event) => {
		const payload: any = event.payload || {};
		if (typeof payload.has_battery === 'boolean') {
			existBattery.value = payload.has_battery;
		}
	});
	await startSNIWatcher();

	document.addEventListener('click', hideContextMenu);
});

onUnmounted(() => {
	unlisten?.();
	unlistenBatteryEvent?.();
	document.removeEventListener('click', hideContextMenu);
});
</script>

<template>
  <div class="tray-panel">
    <TransitionGroup name="tray-list" tag="div" class="flex items-center gap-1">
      <TrayMusicControl />
      <div
        v-for="item in trayItems"
        :key="item.service_name"
        :class="[
          'tray-item group',
          getItemStatusClass(item),
          getItemPulseClass(item),
        ]"
        @click="(e) => handleTrayClick(item, e)"
        @contextmenu.prevent="(e) => handleTrayClick(item, e)"
        :title="item.tooltip || item.title"
      >
        <!-- Icon with loading state -->
        <div class="tray-icon-container">
          <img
            v-if="item.icon_data"
            :src="`data:image/png;base64,${item.icon_data}`"
            :alt="item.title || item.service_name"
            class="tray-icon"
            @error="($event.target as HTMLImageElement).style.display = 'none'"
          />
          <div
            v-else
            class="tray-icon-placeholder"
            :class="{ 'animate-pulse': !item.icon_data }"
          />
        </div>

        <!-- Status indicator -->
        <div v-if="item.status === 'NeedsAttention'" class="status-indicator" />
      </div>
      <TrayIconSound />
      <TrayIconBattery v-if="existBattery" />
      <TrayIconBluetooth />
      <TrayIconNetwork />
    </TransitionGroup>

    <!-- Context Menu -->
    <Teleport to="body">
      <Transition name="context-menu">
        <div
          v-if="contextMenu.visible"
          class="context-menu"
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
              'context-menu-item',
              {
                disabled: !menuItem.enabled,
                separator: menuItem.type === 'separator',
                checked: menuItem.checked,
              },
            ]"
            @click="menuItem.enabled && handleMenuItemClick(menuItem)"
          >
            <span v-if="menuItem.type !== 'separator'" class="menu-label">
              {{ menuItem.label }}
            </span>
            <div v-if="menuItem.checked" class="check-mark">✓</div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
@reference "../../../style.css";
.tray-panel {
  @apply flex items-center gap-1 px-2 h-full;
}

.tray-item {
  @apply relative flex items-center justify-center w-7 h-7 rounded-vsk cursor-pointer;
  @apply transform transition-all duration-300 ease-out;
  @apply hover:bg-white/15 hover:scale-110 hover:rotate-3;
  @apply active:scale-95 active:rotate-0;
}

.tray-item-active {
  @apply bg-white/10 shadow-lg;
}

.tray-item-passive {
  @apply opacity-70 hover:opacity-100;
}

.tray-item-attention {
  @apply bg-red-500/20 shadow-red-500/50;
}

.tray-icon-container {
  @apply relative w-4 h-4 flex items-center justify-center;
}

.tray-icon {
  @apply w-4 h-4 object-contain;
  @apply transition-all duration-300;
  @apply group-hover:brightness-110 group-hover:scale-110;
  filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.3));
}

.tray-icon-placeholder {
  @apply w-4 h-4 bg-gray-500/30 rounded border border-gray-500/50;
}

.status-indicator {
  @apply absolute -top-1 -right-1 w-2 h-2 bg-red-500 rounded-full;
  @apply animate-pulse shadow-lg shadow-red-500/50;
}

.context-menu {
  @apply fixed z-50 bg-white/95 dark:bg-black/95 backdrop-blur-md;
  @apply border border-gray-200/50 dark:border-gray-700/50 rounded-vsk shadow-2xl;
  @apply py-2 min-w-48 max-w-64;
}

.context-menu-item {
  @apply flex items-center justify-between px-4 py-2 text-sm;
  @apply cursor-pointer transition-colors duration-200;
  @apply hover:bg-gray-100/50 dark:hover:bg-gray-800/50;
}

.context-menu-item.disabled {
  @apply opacity-50 cursor-not-allowed;
}

.context-menu-item.separator {
  @apply h-px bg-gray-200/50 dark:bg-gray-700/50 my-1 mx-2 cursor-default;
}

.context-menu-item.checked {
  @apply bg-blue-50/50 dark:bg-blue-900/20;
}

.menu-label {
  @apply flex-1 text-left;
}

.check-mark {
  @apply text-blue-600 dark:text-blue-400 font-bold ml-2;
}

/* Animations */
@keyframes pulse-attention {
  0%,
  100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.7;
    transform: scale(1.05);
  }
}

.animate-pulse-attention {
  animation: pulse-attention 2s infinite ease-in-out;
}

/* Transition Groups */
.tray-list-move {
  transition: transform 0.4s cubic-bezier(0.25, 0.8, 0.25, 1);
}

.tray-list-enter-active {
  transition: all 0.4s cubic-bezier(0.25, 0.8, 0.25, 1);
}

.tray-list-leave-active {
  transition: all 0.3s cubic-bezier(0.55, 0, 0.45, 1);
}

.tray-list-enter-from {
  opacity: 0;
  transform: translateX(-20px) scale(0.8) rotate(-10deg);
}

.tray-list-leave-to {
  opacity: 0;
  transform: translateX(20px) scale(0.8) rotate(10deg);
}

/* Context Menu Transitions */
.context-menu-enter-active {
  transition: all 0.2s cubic-bezier(0.25, 0.8, 0.25, 1);
}

.context-menu-leave-active {
  transition: all 0.15s cubic-bezier(0.55, 0, 0.45, 1);
}

.context-menu-enter-from {
  opacity: 0;
  transform: translateY(-100%) scale(0.95);
}

.context-menu-leave-to {
  opacity: 0;
  transform: translateY(-100%) scale(0.95);
}
</style>
