<script lang="ts" setup>
/**
 * Las tareas del día en el escritorio (vasak-desktop#269): las que vencen hoy o
 * están atrasadas, de las cuentas con calendario.
 *
 * Se piden al montar y cuando el sincronizador trae algo (`CALENDAR_CHANGED`);
 * cuántas filas entran lo decide el alto de la caja. Sin servicio o sin permiso
 * se dice y no se inventa una lista vacía.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import { CALENDAR_CHANGED, calendarTasks, type TasksReply } from '@/services/calendar.service';
import { useBoxSize } from '@/tools/composables/useBoxSize';
import { useSharedEvent } from '@/tools/event.bus';
import { dueToday, isOverdue, rowsFor } from '@/tools/tasks-widget';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const box = ref<HTMLElement | null>(null);
const { height } = useBoxSize(box);

const reply = ref<TasksReply | null>(null);
const now = ref(new Date());

async function load(): Promise<void> {
	try {
		reply.value = await calendarTasks();
		now.value = new Date();
	} catch (error) {
		reply.value = null;
		logError('[widget-tareas] no se pudieron leer las tareas:', error);
	}
}

onMounted(() => void load());
useSharedEvent(CALENDAR_CHANGED, () => void load());

const today = computed(() => dueToday(reply.value, now.value));
const rows = computed(() => rowsFor(height.value));
const shown = computed(() => today.value.slice(0, rows.value));
const hidden = computed(() => today.value.length - shown.value.length);
const unavailable = computed(() => reply.value !== null && reply.value.state !== 'ready');
</script>

<template>
  <div ref="box" class="flex h-full w-full flex-col gap-[2cqmin] overflow-hidden p-[4cqmin]" data-tasks-widget>
    <span class="shrink-0 truncate text-[clamp(0.65rem,5cqmin,0.9rem)] font-semibold text-tx-muted">{{ t('widgets.tasks.heading') }}</span>
    <p v-if="unavailable" class="m-0 truncate text-[clamp(0.65rem,5cqmin,0.9rem)] text-tx-muted">
      {{ t('widgets.tasks.unavailable') }}
    </p>
    <p v-else-if="reply && today.length === 0" class="m-0 truncate text-[clamp(0.65rem,5cqmin,0.9rem)] text-tx-muted">
      {{ t('widgets.tasks.none') }}
    </p>
    <ul v-else class="m-0 min-h-0 flex-1 list-none overflow-hidden p-0">
      <li
        v-for="task in shown"
        :key="task.task_id"
        class="flex min-w-0 items-center gap-[2cqmin] py-[0.5cqmin] text-[clamp(0.7rem,5.5cqmin,1rem)]"
        :class="isOverdue(task, now) ? 'text-status-error' : 'text-tx-main'"
      >
        <span class="h-[1.6cqmin] w-[1.6cqmin] shrink-0 rounded-corner-full bg-primary"></span>
        <span class="truncate">{{ task.title }}</span>
      </li>
      <li v-if="hidden > 0" class="text-[clamp(0.6rem,4.5cqmin,0.85rem)] text-tx-muted">
        {{ t('widgets.tasks.more').replace('{0}', String(hidden)) }}
      </li>
    </ul>
  </div>
</template>
