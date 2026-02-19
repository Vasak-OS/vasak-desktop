<template>
  <div
    class="notification-container flex items-start gap-3 p-3 background rounded-vsk shadow-sm transition-all duration-200 hover:shadow-lg"
    :class="{
      'opacity-75 scale-95': notification.seen,
      'notification-new': !notification.seen,
    }" :data-urgency="notification.urgency?.toLowerCase()">
    <img :src="iconSrc" :alt="notification.app_name" class="w-10 h-10 object-contain" />
    <div class="flex-1 min-w-0">
      <div class="flex items-center justify-between gap-2">
        <h3 class="font-medium truncate">{{ notification.summary }}</h3>
        <button @click="$emit('seen', notification.id)"
          class="close-button flex items-center justify-center w-6 h-6 rounded-full text-gray-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 dark:text-gray-400 dark:hover:text-red-400 transition-all duration-200 transform hover:scale-110">
          <img :src="closeIconSrc" alt="Cerrar" class="w-3 h-3 transition-transform duration-200" />
        </button>
      </div>
      <p class="text-sm text-gray-600 dark:text-gray-300 line-clamp-2">
        {{ notification.body }}
      </p>
      <div class="flex items-center gap-2 mt-1 text-xs text-gray-500 dark:text-gray-400">
        <span>{{ notification.app_name }}</span>
        <span>•</span>
        <span>{{ formatTime(notification.timestamp) }}</span>
      </div>

      <!-- Actions -->
      <div v-if="parsedActions.length > 0" class="flex flex-wrap gap-2 mt-2" @click.stop>
        <ActionButton
          v-for="action in parsedActions"
          :key="action.key"
          :label="action.label"
          variant="secondary"
          custom-class="text-xs"
          @click="() => handleAction(action.key)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, ref } from 'vue';
import { logError } from '@/utils/logger';

const props = defineProps<{
	notification: {
		id: number;
		app_name: string;
		app_icon: string;
		summary: string;
		body: string;
		timestamp: number;
		seen: boolean;
		urgency?: string;
		actions?: string[];
		hints?: { [key: string]: string };
	};
}>();

defineEmits<{
	seen: [id: number];
}>();

const iconSrc = ref('');
const closeIconSrc = ref('');

// Parse standard DBus actions [key, label, key, label...]
const _parsedActions = computed(() => {
	const acts = props.notification.actions || [];
	const result = [];
	for (let i = 0; i < acts.length; i += 2) {
		const key = acts[i];
		const label = acts[i + 1] || key;
		if (key !== 'default') {
			result.push({ key, label });
		}
	}
	return result;
});

function _formatTime(timestamp: number): string {
	const date = new Date(timestamp * 1000);
	return date.toLocaleTimeString();
}

async function _handleAction(action_key: string) {
	try {
		await invoke('invoke_notification_action', {
			id: props.notification.id,
			action_key,
		});
	} catch (error) {
		logError('Error ejecutando acción de notificación:', error);
	}
}

onMounted(async () => {
	try {
		iconSrc.value = await getIconSource(props.notification.app_icon);
		closeIconSrc.value = await getIconSource('window-close-symbolic');
	} catch (error) {
		logError('Error cargando iconos de notificación:', error);
	}
});
</script>

<style scoped>
.notification-container {
  border: 1px solid transparent;
  background: linear-gradient(135deg,
      rgba(255, 255, 255, 0.5),
      rgba(255, 255, 255, 0.3));
  backdrop-filter: blur(10px);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.dark .notification-container {
  background: linear-gradient(135deg, rgba(0, 0, 0, 0.5), rgba(0, 0, 0, 0.3));
}

.notification-container:hover {
  transform: translateY(-2px);
  border-color: rgba(59, 130, 246, 0.3);
}

.notification-new {
  border-left: 3px solid var(--primary-color);
  animation: notification-glow 2s ease-in-out;
}

@keyframes notification-glow {
  0% {
    box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.4);
  }

  50% {
    box-shadow: 0 0 0 8px rgba(59, 130, 246, 0.1);
  }

  100% {
    box-shadow: 0 0 0 0 rgba(59, 130, 246, 0);
  }
}

.close-button:hover img {
  transform: rotate(90deg);
}

.close-button:active {
  transform: scale(0.95);
}

.notification-container img:first-child {
  transition: transform 0.2s ease-in-out;
}

.notification-container:hover img:first-child {
  transform: scale(1.05);
}

.notification-container[data-urgency="critical"] {
  border-left-color: #ef4444;
  background: linear-gradient(135deg,
      rgba(239, 68, 68, 0.1),
      rgba(239, 68, 68, 0.05));
}

.notification-container[data-urgency="critical"].notification-new {
  animation: critical-pulse 1.5s ease-in-out infinite;
}

@keyframes critical-pulse {

  0%,
  100% {
    border-left-color: #ef4444;
  }

  50% {
    border-left-color: #fca5a5;
    box-shadow: 0 0 15px rgba(239, 68, 68, 0.3);
  }
}

.notification-container h3,
.notification-container p,
.notification-container span {
  transition: color 0.2s ease-in-out;
}
</style>
