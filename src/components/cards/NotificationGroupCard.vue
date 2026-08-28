<template>
  <div class="transition-all duration-200 ease-in-out">
    <!-- Encabezado del grupo.
         Sin cambio de fondo al pasar el mouse: el barrido diagonal y el
         degradado a azul y violeta que había acá no son del sistema y se veían
         de juguete. Lo que indica que la fila responde al clic es el cursor y
         el giro de la flecha, que alcanza. -->
    <div
      class="group/grupo flex items-center gap-2 px-2 py-1.5 bg-ui-surface rounded-t-corner cursor-pointer"
      @click="toggleExpanded" :class="{ 'rounded-corner': !isExpanded }">
      <img :src="iconSrc" :alt="group.app_name" class="w-5 h-5 shrink-0 object-contain" />

      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2">
          <h3 class="text-sm font-medium text-tx-main truncate">
            {{ group.app_name }}
          </h3>
          <span
            class="inline-flex items-center justify-center min-w-4 h-4 px-1 text-[11px] font-medium rounded-full"
            :class="group.has_unread ? 'bg-primary text-tx-on-primary' : 'bg-ui-bg text-tx-muted'">
            {{ group.count }}
          </span>
        </div>
        <p class="text-xs text-tx-muted truncate">
          {{ formatGroupSummary() }}
        </p>
      </div>

      <div class="flex items-center gap-1 shrink-0">
        <span class="text-[11px] text-tx-muted">
          {{ formatTime(group.latest_timestamp) }}
        </span>
        <button
          type="button"
          :title="t('components.NotificationGroupCard.removeGroup')"
          class="flex items-center justify-center w-4 h-4 rounded-full text-tx-muted opacity-0 transition-opacity duration-200 group-hover/grupo:opacity-100 focus-visible:opacity-100 hover:text-status-error"
          @click.stop="removeAllFromGroup"
        >
          <img :src="closeIconSrc" :alt="t('components.NotificationGroupCard.removeGroup')" class="w-2.5 h-2.5" />
        </button>
        <div class="w-4 h-4 flex items-center justify-center text-tx-muted transition-transform duration-200"
          :class="{ 'rotate-180': isExpanded }">
          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
          </svg>
        </div>
      </div>
    </div>

    <Transition enter-active-class="transition-all duration-300 ease-[cubic-bezier(0.4,0,0.2,1)] overflow-hidden"
      leave-active-class="transition-all duration-300 ease-[cubic-bezier(0.4,0,0.2,1)] overflow-hidden"
      enter-from-class="h-0 opacity-0" leave-to-class="h-0 opacity-0" @enter="onEnter" @leave="onLeave">
      <div v-show="isExpanded"
        class="notifications-list bg-ui-bg/60 rounded-b-corner">
        <TransitionGroup move-class="transition-transform duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]"
          enter-active-class="transition-all duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]"
          leave-active-class="transition-all duration-200 ease-[cubic-bezier(0.4,0,0.2,1)]"
          enter-from-class="opacity-0 translate-x-5" leave-to-class="opacity-0 -translate-x-5" tag="div">
          <NotificationCard v-for="notification in group.notifications" :key="notification.id"
            :notification="notification" @seen="(id: number) => $emit('remove', id)"
            class="border-b border-ui-border last:border-b-0" />
        </TransitionGroup>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import NotificationCard from '@/components/cards/NotificationCard.vue';
import { useIcons } from '@/tools/composables/useReactiveIcon';

const { t } = useI18n();

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
const { iconSrc, closeIconSrc } = useIcons({
	iconSrc: computed(() => props.group.app_icon),
	closeIconSrc: 'window-close-symbolic',
});

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
		const label =
			unreadCount === 1
				? t('components.NotificationGroupCard.unreadOne')
				: t('components.NotificationGroupCard.unreadMany');
		return label.replace('{0}', String(unreadCount));
	}
	return props.group.notifications[0]?.summary || t('components.NotificationGroupCard.empty');
}

function formatTime(timestamp: number) {
	const date = new Date(timestamp * 1000);
	const now = new Date();
	const diffMinutes = Math.floor((now.getTime() - date.getTime()) / (1000 * 60));

	if (diffMinutes < 1) return t('components.NotificationGroupCard.now');
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

onMounted(() => {
	if (shouldAutoExpand.value) {
		isExpanded.value = true;
	}
});
</script>

