<template>
  <div
    class="flex flex-col gap-2 w-full h-[calc(100vh-390px)] overflow-y-auto p-4"
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
        class="text-xs px-2 py-1 background rounded-vsk hover:bg-vsk-primary transition-colors"
      >
        Limpiar todo
      </button>
    </div>

    <div
      v-if="groupedNotifications.length === 0"
      class="text-center text-gray-500 dark:text-gray-400 py-8"
    >
      <div class="opacity-60">🔔</div>
      <p class="mt-2">No hay notificaciones</p>
    </div>

    <TransitionGroup name="notification" tag="div" class="flex flex-col gap-3">
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
import { ref, onMounted, onUnmounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import NotificationGroupCard from "@/components/cards/NotificationGroupCard.vue";
import {
  Notification,
  NotificationGroupData,
} from "@/interfaces/notifications";

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

    const group = groups.get(appName)!;
    group.notifications.push(notification);
    group.count = group.notifications.length;
    group.latest_timestamp = Math.max(
      group.latest_timestamp,
      notification.timestamp
    );
    group.has_unread = group.has_unread || !notification.seen;
  });

  // Ordenar grupos por timestamp más reciente
  return Array.from(groups.values()).sort(
    (a, b) => b.latest_timestamp - a.latest_timestamp
  );
});

async function loadNotifications() {
  try {
    notifications.value = await invoke("get_all_notifications");
  } catch (error) {
    console.error("Error loading notifications:", error);
  }
}

async function removeNotification(id: number) {
  try {
    await invoke("delete_notification", { id });
    // No necesitamos actualizar la lista local aquí porque el evento lo hará
  } catch (error) {
    console.error("Error removing notification:", error);
  }
}

async function clearAllNotifications() {
  try {
    await invoke("clear_notifications");
  } catch (error) {
    console.error("Error clearing all notifications:", error);
  }
}

onMounted(async () => {
  await loadNotifications();

  unlistenNotifications = await listen("notifications-updated", (event) => {
    notifications.value = event.payload as Notification[];
  });
});

onUnmounted(() => {
  if (unlistenNotifications) {
    unlistenNotifications();
  }
});
</script>

<style scoped>
/* Animaciones para entrada y salida de notificaciones */
.notification-enter-active {
  transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.notification-leave-active {
  transition: all 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94);
}

.notification-enter-from {
  opacity: 0;
  transform: translateX(100%) scale(0.9);
}

.notification-leave-to {
  opacity: 0;
  transform: translateX(-30%) scale(0.95);
}

.notification-move {
  transition: transform 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94);
}

/* Animación de entrada para el elemento */
.notification-item {
  transition: all 0.2s ease-in-out;
}

.notification-item:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

/* Animación de pulsación para nuevas notificaciones */
@keyframes pulse-notification {
  0%,
  100% {
    transform: scale(1);
  }
  50% {
    transform: scale(1.02);
  }
}

.notification-enter-active .notification-item {
  animation: pulse-notification 0.6s ease-in-out;
}

/* Mejora del estado vacío */
.text-center {
  transition: opacity 0.3s ease-in-out;
}
</style>
