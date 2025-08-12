<template>
  <div
    class="flex items-start gap-3 p-3 bg-white/50 dark:bg-black/50 rounded-lg shadow-sm hover:shadow-md transition-shadow"
    :class="{ 'opacity-75': notification.seen }"
  >
    <img
      :src="iconSrc"
      :alt="notification.app_name"
      class="w-10 h-10 object-contain"
    />
    <div class="flex-1 min-w-0">
      <div class="flex items-center justify-between gap-2">
        <h3 class="font-medium truncate">{{ notification.summary }}</h3>
        <button
          @click="$emit('seen', notification.id)"
          class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
        >
          <img :src="closeIconSrc" alt="Cerrar" class="w-4 h-4" />
        </button>
      </div>
      <p class="text-sm text-gray-600 dark:text-gray-300 line-clamp-2">
        {{ notification.body }}
      </p>
      <div
        class="flex items-center gap-2 mt-1 text-xs text-gray-500 dark:text-gray-400"
      >
        <span>{{ notification.app_name }}</span>
        <span>•</span>
        <span>{{ formatTime(notification.timestamp) }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, Ref } from "vue";
import { getIconSource } from "@vasakgroup/plugin-vicons";

const props = defineProps<{
  notification: {
    id: number;
    app_name: string;
    app_icon: string;
    summary: string;
    body: string;
    timestamp: number;
    seen: boolean;
  };
}>();

defineEmits<{
  (e: "seen", id: number): void;
}>();

const iconSrc: Ref<string> = ref("");
const closeIconSrc: Ref<string> = ref("");

function formatTime(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toLocaleTimeString();
}

onMounted(async () => {
  try {
    iconSrc.value = await getIconSource(props.notification.app_icon);
    closeIconSrc.value = await getIconSource("window-close-symbolic");
  } catch (error) {
    console.error("Error loading icons:", error);
  }
});
</script>
