<template>
  <div
    class="flex flex-col gap-2 w-full h-[calc(100vh-550px)] overflow-y-auto p-4"
  >
    <div
      class="flex items-center justify-between mb-2"
      v-if="groupedNotifications.length > 0"
    >
      <span class="text-sm text-gray-600 dark:text-gray-300">
        {{ notifications.length }} Notification{{
          notifications.length !== 1 ? "s" : ""
        }}
        <span class="text-xs opacity-75">
          ({{ groupedNotifications.length }} App{{
            groupedNotifications.length !== 1 ? "s" : ""
          }})
        </span>
      </span>
      <button
        @click="clearAllNotifications"
        class="text-xs px-4 py-2 bg-primary rounded-corner hover:bg-primary/80 transition-colors"
      >
        Limpiar todo
      </button>
    </div>

    <div
      v-if="groupedNotifications.length === 0"
      class="text-center transition-opacity duration-300 ease-in-out text-tx-muted dark:text-tx-muted-dark py-8"
    >
      <div class="opacity-60">🔔</div>
      <p class="mt-2">No hay notificaciones</p>
    </div>

    <TransitionGroup move-class="transition-transform duration-300 ease-[cubic-bezier(0.25,0.46,0.45,0.94)]" enter-active-class="transition-all duration-400 ease-[cubic-bezier(0.34,1.56,0.64,1)] [&>.notification-item]:animate-pulse-notification" leave-active-class="transition-all duration-300 ease-[cubic-bezier(0.25,0.46,0.45,0.94)]" enter-from-class="opacity-0 translate-x-full scale-90" leave-to-class="opacity-0 translate-x-[-30%] scale-95" tag="div" class="flex flex-col gap-3">
      <NotificationGroupCard
        v-for="group in groupedNotifications"
        :key="group.app_name"
        :group="group"
        @remove="removeNotification"
        class="notification-group"
      />
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { listen } from '@tauri-apps/api/event';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import NotificationGroupCard from '@/components/cards/NotificationGroupCard.vue';
import type { Notification, NotificationGroupData } from '@/interfaces/notifications';
import {
	clearNotifications,
	deleteNotification,
	getAllNotifications,
} from '@/services/notification.service';

const notifications = ref<Notification[]>([]);
let unlistenNotifications: (() => void) | null = null;

// Computed para agrupar notificaciones por aplicación
const groupedNotifications = computed<NotificationGroupData[]>(() => {
	const groups = new Map<string, NotificationGroupData>();

	notifications.value.forEach((notification) => {
		const appName = notification.app_name;

		if (!groups.has(appName)) {
			groups.set(appName, {
				app_name: appName,
				app_icon: notification.app_icon,
				notifications: [],
				count: 0,
				latest_timestamp: 0,
				has_unread: false,
			});
		}

		// biome-ignore lint/style/noNonNullAssertion: <Is necessary for dinamic grouping>
		const group = groups.get(appName)!;
		group.notifications.push(notification);
		group.count = group.notifications.length;
		group.latest_timestamp = Math.max(group.latest_timestamp, notification.timestamp);
		group.has_unread = group.has_unread || !notification.seen;
	});

	return Array.from(groups.values()).sort((a, b) => b.latest_timestamp - a.latest_timestamp);
});

async function loadNotifications() {
	try {
		notifications.value = await getAllNotifications();
	} catch (error) {
		console.error('Error loading notifications:', error);
	}
}

async function removeNotification(id: number) {
	try {
		await deleteNotification({ id });
		// No necesitamos actualizar la lista local aquí porque el evento lo hará
	} catch (error) {
		console.error('Error removing notification:', error);
	}
}

async function clearAllNotifications() {
	try {
		await clearNotifications();
	} catch (error) {
		console.error('Error clearing all notifications:', error);
	}
}

onMounted(async () => {
	await loadNotifications();

	unlistenNotifications = await listen('notifications-updated', (event) => {
		notifications.value = event.payload as Notification[];
	});
});

onUnmounted(() => {
	if (unlistenNotifications) {
		unlistenNotifications();
	}
});
</script>

