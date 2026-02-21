<template>
  <div class="notification-group-container">
    <!-- Header del grupo -->
    <div
      class="group-header flex items-center gap-3 p-3 bg-gradient-to-r from-blue-50/50 to-purple-50/50 dark:from-blue-900/20 dark:to-purple-900/20 rounded-t-vsk border-l-4 border-blue-500 cursor-pointer transition-all duration-200 hover:bg-gradient-to-r hover:from-blue-100/50 hover:to-purple-100/50 dark:hover:from-blue-800/30 dark:hover:to-purple-800/30"
      @click="toggleExpanded" :class="{
        'rounded-corner': !isExpanded,
        'shadow-sm': group.has_unread,
      }">
      <img :src="iconSrc" :alt="group.app_name" class="w-8 h-8 object-contain transition-transform duration-200"
        :class="{ 'scale-110': group.has_unread }" />

      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2">
          <h3 class="font-medium text-gray-900 dark:text-gray-100">
            {{ group.app_name }}
          </h3>
          <span
            class="inline-flex items-center justify-center w-5 h-5 text-xs font-medium rounded-full transition-all duration-200"
            :class="{
              'bg-blue-500 text-white': group.has_unread,
              'bg-gray-300 text-gray-600 dark:bg-gray-600 dark:text-gray-300':
                !group.has_unread,
            }">
            {{ group.count }}
          </span>
        </div>
        <p class="text-sm text-gray-600 dark:text-gray-400">
          {{ formatGroupSummary() }}
        </p>
      </div>

      <div class="flex items-center gap-2">
        <span class="text-xs text-gray-500 dark:text-gray-400">
          {{ formatTime(group.latest_timestamp) }}
        </span>
        <ActionButton
          label=""
          :iconSrc="closeIconSrc"
          iconAlt="Eliminar grupo"
          size="sm"
          variant="secondary"
          :stopPropagation="true"
          customClass="bg-transparent text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 border border-transparent"
          @click="removeAllFromGroup"
        />
        <div class="w-5 h-5 flex items-center justify-center transition-transform duration-200"
          :class="{ 'rotate-180': isExpanded }">
          <svg class="w-3 h-3 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
          </svg>
        </div>
      </div>
    </div>

    <Transition name="expand" @enter="onEnter" @leave="onLeave">
      <div v-show="isExpanded"
        class="notifications-list bg-white/30 dark:bg-black/30 rounded-b-vsk border-l-4 border-blue-500/30">
        <TransitionGroup name="notification-item" tag="div">
          <NotificationCard v-for="notification in group.notifications" :key="notification.id"
            :notification="notification" @seen="(id: number) => $emit('remove', id)"
            class="border-b border-gray-200/50 dark:border-gray-700/50 last:border-b-0" />
        </TransitionGroup>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, ref } from 'vue';
import NotificationCard from '@/components/cards/NotificationCard.vue';
import { logError } from '@/utils/logger';
import ActionButton from '../buttons/ActionButton.vue';

interface Notification {
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
}

interface NotificationGroupData {
	app_name: string;
	app_icon: string;
	notifications: Notification[];
	count: number;
	latest_timestamp: number;
	has_unread: boolean;
}

const props = defineProps<{
	group: NotificationGroupData;
}>();

const emit = defineEmits<{
	remove: [id: number];
}>();

const isExpanded = ref(false);
const iconSrc = ref('');
const closeIconSrc = ref('');

// Auto-expandir si hay notificaciones no leídas
const shouldAutoExpand = computed(() => {
	return props.group.has_unread && props.group.count <= 3;
});

function toggleExpanded() {
	isExpanded.value = !isExpanded.value;
}

function formatGroupSummary() {
	const unreadCount = props.group.notifications.filter((n) => !n.seen).length;
	if (unreadCount > 0) {
		return `${unreadCount} nueva${unreadCount === 1 ? '' : 's'}`;
	}
	return props.group.notifications[0]?.summary || 'Sin notificaciones';
}

function formatTime(timestamp: number) {
	const date = new Date(timestamp * 1000);
	const now = new Date();
	const diffMinutes = Math.floor((now.getTime() - date.getTime()) / (1000 * 60));

	if (diffMinutes < 1) return 'ahora';
	if (diffMinutes < 60) return `${diffMinutes}m`;
	if (diffMinutes < 1440) return `${Math.floor(diffMinutes / 60)}h`;
	return date.toLocaleDateString();
}

function removeAllFromGroup() {
	props.group.notifications.forEach((notification) => {
		emit('remove', notification.id as number);
	});
}

function onEnter(el: Element) {
	const element = el as HTMLElement;
	element.style.height = '0';
	element.style.overflow = 'hidden';

	requestAnimationFrame(() => {
		element.style.height = `${element.scrollHeight}px`;
		element.style.transition = 'height 0.3s cubic-bezier(0.4, 0, 0.2, 1)';
	});
}

function onLeave(el: Element) {
	const element = el as HTMLElement;
	element.style.height = `${element.scrollHeight}px`;
	element.style.overflow = 'hidden';

	requestAnimationFrame(() => {
		element.style.height = '0';
		element.style.transition = 'height 0.3s cubic-bezier(0.4, 0, 0.2, 1)';
	});
}

onMounted(async () => {
	try {
		iconSrc.value = await getIconSource(props.group.app_icon);
		closeIconSrc.value = await getIconSource('window-close-symbolic');

		// Auto-expandir si cumple condiciones
		if (shouldAutoExpand.value) {
			isExpanded.value = true;
		}
	} catch (error) {
		logError('Error cargando iconos de notificaciones:', error);
	}
});
</script>

<style scoped>
.notification-group-container {
  transition: all 0.2s ease;
}

.group-header {
  position: relative;
  overflow: hidden;
}

.group-header::before {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(45deg,
      transparent 49%,
      rgba(255, 255, 255, 0.1) 50%,
      transparent 51%);
  transform: translateX(-100%);
  transition: transform 0.6s ease;
}

.group-header:hover::before {
  transform: translateX(100%);
}

.expand-enter-active,
.expand-leave-active {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;
}

.expand-enter-from,
.expand-leave-to {
  height: 0;
  opacity: 0;
}

.notification-item-enter-active {
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.notification-item-leave-active {
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.notification-item-enter-from {
  opacity: 0;
  transform: translateX(20px);
}

.notification-item-leave-to {
  opacity: 0;
  transform: translateX(-20px);
}

.notification-item-move {
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}
</style>
