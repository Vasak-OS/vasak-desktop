<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { onMounted, onUnmounted, type Ref, ref } from 'vue';
import menuIcon from '@/assets/vectors/icon.svg';
import TrayBarArea from '@/components/areas/panel/TrayBarArea.vue';
import WindowsArea from '@/components/areas/panel/WindowsArea.vue';
import PanelClockwidget from '@/components/widgets/PanelClockwidget.vue';
import { getAllNotifications } from '@/services/notification.service';
import {
	openFileManagerWindow,
	toggleConfigApp,
	toggleControlCenter,
	toggleMenu,
} from '@/services/window.service';
import { logError } from '@/utils/logger';

const notifyIcon: Ref<string> = ref('');
const configIcon: Ref<string> = ref('');
const fileManagerIcon: Ref<string> = ref('');

const notifications = ref<Notification[]>([]);
const hasNewNotifications = ref(false);
let unlistenNotifications: Ref<(() => void) | null> = ref(null);

const setIcons = async () => {
	try {
		notifyIcon.value = await getIconSource('preferences-desktop-notification');
		configIcon.value = await getIconSource('preferences-system');
		fileManagerIcon.value = await getIconSource('system-file-manager');
	} catch (err) {
		logError('Error finding icons:', err);
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
		await toggleConfigApp();
	} catch (error) {
		logError('Error al abrir config:', error);
	}
};

const openFileManager = async () => {
	try {
		await openFileManagerWindow({} as any);
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
	setIcons();
	await loadNotifications();

	unlistenNotifications.value = await listen('notifications-updated', (event) => {
		const newNotifications = event.payload as Notification[];
		hasNewNotifications.value = newNotifications.length > notifications.value.length;
		notifications.value = newNotifications;

		// Reset animation after a short delay
		if (hasNewNotifications.value) {
			setTimeout(() => {
				hasNewNotifications.value = false;
			}, 1000);
		}
	});
});

onUnmounted(() => {
	if (unlistenNotifications.value) {
		unlistenNotifications.value();
	}
});
</script>

<template>
  <nav class="flex justify-between items-center mx-1 h-9 mt-0.5 p-1 rounded-corner background">
    <div class="flex items-center gap-1">
      <img :src="menuIcon" alt="Menu" @click="openMenu" class="h-6 w-6 cursor-pointer p-0.5 rounded-corner hover:bg-primary/80 dark:hover:bg-primary-dark/80 transform hover:scale-110 active:scale-95 ease-in-out" />
      <img
        :src="configIcon"
        alt="Config"
        @click="openConfig"
        class="h-6 w-6 cursor-pointer p-0.5 rounded-corner hover:bg-primary/80 dark:hover:bg-primary-dark/80 transform hover:scale-110 active:scale-95 ease-in-out"
      />
      <img
        :src="fileManagerIcon"
        alt="Files"
        @click="openFileManager"
        class="h-6 w-6 cursor-pointer p-0.5 rounded-corner hover:bg-primary/80 dark:hover:bg-primary-dark/80 transform hover:scale-110 active:scale-95 ease-in-out"
      />
    </div>
    <WindowsArea />
    <div class="flex content-center items-center">
      <TrayBarArea />
      <PanelClockwidget />
      <div class="relative cursor-pointer" @click="openNotificationCenter">
        <img
          :src="notifyIcon"
          alt="Notifications"
          class="h-6 w-6 cursor-pointer p-0.5 rounded-corner hover:bg-primary/80 dark:hover:bg-primary-dark/80 transform hover:scale-110 active:scale-95 ease-in-out"
          :class="{ 'animate-bell-shake': hasNewNotifications }"
        />
        <div v-if="notifications.length > 0" class="absolute -top-0.5 -right-0.5 bg-primary dark:bg-primary-dark text-tx-on-primary dark:text-tx-on-primary-dark rounded-full min-w-3 h-3 flex items-center justify-center text-[8px] font-semibold leading-none px-[2px]">
          {{ notifications.length > 99 ? "99+" : notifications.length }}
        </div>
      </div>
    </div>
  </nav>
</template>

