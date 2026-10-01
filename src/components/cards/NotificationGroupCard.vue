<template>
  <!-- La cabecera que despliega es `Disclosure` de la librería (vue-libvasak
       2.2.0): un `<button>` de verdad con `aria-expanded` y `aria-controls`,
       y no un `div` con papel de botón. El de descartar el grupo entero no
       puede ir adentro de ese botón —un botón dentro de otro no es HTML
       válido—, así que va al lado, encima del hueco que le deja la cabecera. -->
  <div class="group/group relative min-w-0">
    <Disclosure v-model:open="isExpanded" variant="card" :title="group.app_name">
      <template #title>
        <span class="flex min-w-0 items-center gap-2">
          <ThemeIcon :name="group.app_icon" :size="20" :alt="group.app_name" class="shrink-0 object-contain" />
          <span class="flex min-w-0 flex-1 flex-col">
            <span class="flex min-w-0 items-center gap-2">
              <span class="truncate">{{ group.app_name }}</span>
              <Badge
                :label="group.count"
                :tone="group.has_unread ? 'accent' : 'neutral'"
                :variant="group.has_unread ? 'solid' : 'soft'"
              />
            </span>
            <span class="truncate text-label-xs font-normal text-tx-muted">{{ formatGroupSummary() }}</span>
          </span>
        </span>
      </template>
      <template #meta>
        <span class="text-label-xs">{{ formatTime(group.latest_timestamp) }}</span>
        <!-- El lugar del botón de descartar, que va afuera de la cabecera. -->
        <span aria-hidden="true" class="w-6 shrink-0" />
      </template>

      <TransitionGroup
        move-class="transition-transform duration-300 ease-ui"
        enter-active-class="transition-[opacity,translate] duration-300 ease-ui-out"
        leave-active-class="transition-[opacity,translate] duration-200 ease-ui"
        enter-from-class="opacity-0 translate-x-4"
        leave-to-class="opacity-0 -translate-x-4"
        tag="div"
        class="-m-2 flex flex-col"
      >
        <NotificationCard
          v-for="notification in group.notifications"
          :key="notification.id"
          :notification="notification"
          class="border-b border-ui-line-weak last:border-b-0"
          @seen="(id: number) => $emit('remove', id)"
        />
      </TransitionGroup>
    </Disclosure>

    <span class="absolute top-2 right-3 flex min-h-8 items-center">
      <ActionButton
        label=""
        icon="window-close-symbolic"
        :icon-alt="t('components.NotificationGroupCard.removeGroup')"
        :title="t('components.NotificationGroupCard.removeGroup')"
        variant="ghost"
        size="sm"
        custom-class="opacity-0 group-hover/group:opacity-100 focus-visible:opacity-100"
        @click="removeAllFromGroup"
      />
    </span>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton, Badge, Disclosure, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import NotificationCard from '@/components/cards/NotificationCard.vue';

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
// Auto-expandir si hay notificaciones no leídas
const shouldAutoExpand = computed(() => {
	return props.group.has_unread && props.group.count <= 3;
});

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

onMounted(() => {
	if (shouldAutoExpand.value) {
		isExpanded.value = true;
	}
});
</script>

