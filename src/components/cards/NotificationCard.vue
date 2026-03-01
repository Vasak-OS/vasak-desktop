<template>
  <div
    class="border border-transparent bg-[linear-gradient(135deg,rgba(255,255,255,0.5),rgba(255,255,255,0.3))] dark:bg-[linear-gradient(135deg,rgba(0,0,0,0.5),rgba(0,0,0,0.3))] backdrop-blur-md transition-all duration-300 ease-[cubic-bezier(0.4,0,0.2,1)] hover:-translate-y-[2px] hover:border-blue-500/30 group/nc flex items-start gap-3 p-3 background rounded-corner shadow-sm transition-all duration-200 hover:shadow-lg"
    :class="{
      'opacity-75 scale-95': notification.seen,
      'border-l-4 border-l-primary animate-notification-glow': !notification.seen,
    }" :data-urgency="notification.urgency?.toLowerCase()">
    <img :src="iconSrc" :alt="notification.app_name" class="w-10 h-10 object-contain transition-transform duration-200 ease-in-out group-hover/nc:scale-105" />
    <div class="flex-1 min-w-0">
      <div class="flex items-center justify-between gap-2">
        <h3 class="font-medium truncate">{{ notification.summary }}</h3>
        <button @click="$emit('seen', notification.id)"
          class="close-button group/close active:scale-95 flex items-center justify-center w-6 h-6 rounded-full text-gray-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 dark:text-gray-400 dark:hover:text-red-400 transition-all duration-200 transform hover:scale-110">
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
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, ref } from 'vue';
import { invokeNotificationAction } from '@/services/notification.service';
import { logError } from '@/utils/logger';
import ActionButton from '../buttons/ActionButton.vue';

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
const parsedActions = computed(() => {
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

function formatTime(timestamp: number): string {
	const date = new Date(timestamp * 1000);
	return date.toLocaleTimeString();
}

async function handleAction(action_key: string) {
	try {
		await invokeNotificationAction({
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

