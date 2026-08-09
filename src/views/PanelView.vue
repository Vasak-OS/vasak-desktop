<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { emit } from '@tauri-apps/api/event';
import { Command } from '@tauri-apps/plugin-shell';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { onMounted, ref } from 'vue';
import TrayBarArea from '@/components/areas/panel/TrayBarArea.vue';
import WindowsArea from '@/components/areas/panel/WindowsArea.vue';
import PanelClockwidget from '@/components/widgets/PanelClockwidget.vue';
import type { Notification as AppNotification, NotificationDelta } from '@/interfaces/notifications';
import { getAllNotifications } from '@/services/notification.service';
import { toggleControlCenter, toggleMenu } from '@/services/window.service';
import { useIcons } from '@/tools/composables/useReactiveIcon';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const notifications = ref<AppNotification[]>([]);
const hasNewNotifications = ref(false);
let notificationResetTimer: ReturnType<typeof setTimeout> | undefined;

const { menuIcon, notifyIcon, configIcon, fileManagerIcon } = useIcons({
	menuIcon: 'start-here',
	notifyIcon: 'preferences-desktop-notification',
	configIcon: 'preferences-system',
	fileManagerIcon: 'system-file-manager',
});

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
});

useSharedEvent<NotificationDelta>('notification-delta', (delta) => {
	switch (delta.action) {
		case 'added':
			notifications.value.unshift(delta.notification);
			if (delta.dropped_id != null) {
				notifications.value = notifications.value.filter((n) => n.id !== delta.dropped_id);
			}
			hasNewNotifications.value = true;
			clearTimeout(notificationResetTimer);
			notificationResetTimer = setTimeout(() => {
				hasNewNotifications.value = false;
			}, 1000);
			break;
		case 'removed':
			notifications.value = notifications.value.filter((n) => n.id !== delta.id);
			break;
		case 'batch_update':
			if (delta.added.length > 0) {
				notifications.value = [...delta.added, ...notifications.value];
			}
			if (delta.removed.length > 0) {
				const removedSet = new Set(delta.removed);
				notifications.value = notifications.value.filter((n) => !removedSet.has(n.id));
			}
			if (delta.added.length > 0) {
				hasNewNotifications.value = true;
				clearTimeout(notificationResetTimer);
				notificationResetTimer = setTimeout(() => {
					hasNewNotifications.value = false;
				}, 1000);
			}
			break;
		case 'cleared':
			notifications.value = [];
			break;
	}
});
</script>

<template>
	<nav class="relative z-20 flex w-[calc(100%-8px)] justify-between items-center mx-1 h-9 mt-0.5 overflow-hidden p-1 rounded-corner bg-ui-bg/80 border border-ui-border/80 px-3">
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

