<template>
  <!-- El encabezado queda fijo y la lista se desplaza sola: con el scroll en
       todo el bloque, unas cuantas notificaciones empujaban los controles del
       centro —música, brillo, volumen— fuera de la pantalla. -->
  <div class="flex flex-col w-full min-h-0">
    <div
      class="flex shrink-0 items-center justify-between gap-2 px-1 pb-2"
      v-if="groupedNotifications.length > 0"
    >
      <span class="text-sm text-tx-main font-medium">
        {{ notifications.length }}
        {{
          notifications.length === 1
            ? t('components.NotificationArea.notificationOne')
            : t('components.NotificationArea.notificationMany')
        }}
        <span class="text-xs opacity-75">
          ({{ groupedNotifications.length }}
          {{
            groupedNotifications.length === 1
              ? t('components.NotificationArea.appOne')
              : t('components.NotificationArea.appMany')
          }})
        </span>
      </span>
      <button
        @click="clearAllNotifications"
        class="shrink-0 text-xs px-3 py-1 bg-primary text-tx-on-primary rounded-corner hover:bg-primary/80 transition-colors"
      >
        {{ t('components.NotificationArea.clearAll') }}
      </button>
    </div>

    <div
      v-if="groupedNotifications.length === 0"
      class="text-center transition-opacity duration-300 ease-in-out text-tx-muted py-6"
    >
      <img :src="emptyIcon" alt="" class="w-6 h-6 opacity-60 mx-auto" />
      <p class="mt-1 text-sm">{{ t('components.NotificationArea.empty') }}</p>
    </div>

    <TransitionGroup move-class="transition-transform duration-300 ease-[cubic-bezier(0.25,0.46,0.45,0.94)]" enter-active-class="transition-all duration-400 ease-[cubic-bezier(0.34,1.56,0.64,1)]" leave-active-class="transition-all duration-300 ease-[cubic-bezier(0.25,0.46,0.45,0.94)]" enter-from-class="opacity-0 translate-x-full scale-90" leave-to-class="opacity-0 translate-x-[-30%] scale-95" tag="div" class="flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto overflow-x-hidden pr-1">
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
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import { useSharedEvent } from '@/tools/event.bus';
import { agruparNotificaciones } from '@/tools/notificaciones';

const { t } = useI18n();

const notifications = ref<Notification[]>([]);
const emptyIcon = useSymbol('preferences-desktop-notification');

const groupedNotifications = computed<NotificationGroupData[]>(() =>
	agruparNotificaciones(notifications.value)
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

