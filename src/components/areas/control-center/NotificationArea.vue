<template>
  <!-- El encabezado queda fijo y la lista se desplaza sola: con el scroll en
       todo el bloque, unas cuantas notificaciones empujaban los controles del
       centro —música, brillo, volumen— fuera de la pantalla. -->
  <div class="flex flex-col w-full min-h-0">
    <div
      class="flex shrink-0 items-center justify-between gap-2 px-1 pb-2"
      v-if="groupedNotifications.length > 0"
    >
      <span class="text-label-m text-tx-main font-medium">
        {{ notifications.length }}
        {{
          notifications.length === 1
            ? t('components.NotificationArea.notificationOne')
            : t('components.NotificationArea.notificationMany')
        }}
        <span class="text-label-xs opacity-75">
          ({{ groupedNotifications.length }}
          {{
            groupedNotifications.length === 1
              ? t('components.NotificationArea.appOne')
              : t('components.NotificationArea.appMany')
          }})
        </span>
      </span>
      <ActionButton
        :label="t('components.NotificationArea.clearAll')"
        size="sm"
        custom-class="shrink-0"
        @click="clearAllNotifications"
      />
    </div>

    <EmptyState
      v-if="groupedNotifications.length === 0"
      :title="t('components.NotificationArea.empty')"
      icon="preferences-desktop-notification"
      icon-type="symbol"
      size="sm"
    />

    <TransitionGroup move-class="transition-transform duration-300 ease-ui" enter-active-class="transition-[opacity,translate] duration-300 ease-ui-out" leave-active-class="transition-[opacity,translate] duration-200 ease-ui" enter-from-class="opacity-0 translate-x-4" leave-to-class="opacity-0 -translate-x-4" tag="div" class="flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto overflow-x-hidden pr-1">
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
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton, EmptyState } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import NotificationGroupCard from '@/components/cards/NotificationGroupCard.vue';
import type {
	Notification,
	NotificationDelta,
	NotificationGroupData,
} from '@/interfaces/notifications';
import {
	clearNotifications,
	deleteNotification,
	getAllNotifications,
} from '@/services/notification.service';
import { useSharedEvent } from '@/tools/event.bus';
import { groupNotifications } from '@/tools/notifications';

const { t } = useI18n();

const notifications = ref<Notification[]>([]);

const groupedNotifications = computed<NotificationGroupData[]>(() =>
	groupNotifications(notifications.value)
);

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
});

// Una foto entera reemplaza la lista de una sola vez. Nada de vaciarla primero:
// entre el vaciado y el relleno Vue alcanza a dibujar, y las notificaciones que
// sobrevivían a un borrado se desmontaban y volvían a montarse repitiendo la
// animación de entrada. Asignando una vez, las claves que siguen estando se
// reconocen y sólo se anima lo que de verdad entró o salió.
useSharedEvent<NotificationDelta>('notification-delta', (delta) => {
	notifications.value = delta.items;
});
</script>

