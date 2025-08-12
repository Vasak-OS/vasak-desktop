<template>
  <div class="flex flex-col gap-2 w-full max-h-96 overflow-y-auto p-4">
    <div
      v-if="notifications.length === 0"
      class="text-center text-gray-500 dark:text-gray-400"
    >
      No hay notificaciones
    </div>
    <NotificationCard
      v-for="notification in notifications"
      :key="notification.id"
      :notification="notification"
      @seen="markAsSeen"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import NotificationCard from "@/components/cards/NotificationCard.vue";

interface Notification {
  id: number;
  app_name: string;
  app_icon: string;
  summary: string;
  body: string;
  timestamp: number;
  seen: boolean;
}

const notifications: Ref<Notification[]> = ref<Notification[]>([]);

async function loadNotifications() {
  try {
    notifications.value = await invoke("get_unread_notifications");
  } catch (error) {
    console.error("Error loading notifications:", error);
  }
}

async function markAsSeen(id: number) {
  try {
    await invoke("mark_notification_as_seen", { id });
    notifications.value = notifications.value.filter((n) => n.id !== id);
  } catch (error) {
    console.error("Error marking notification as seen:", error);
  }
}

onMounted(() => {
  loadNotifications();
  const interval = setInterval(loadNotifications, 2000);
  onUnmounted(() => clearInterval(interval));
});
</script>
