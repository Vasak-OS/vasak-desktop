<script lang="ts" setup>
/**
 * El calendario del escritorio (vasak-desktop#269): el mes con los días que
 * tienen eventos y, si la celda tiene alto, los eventos de hoy debajo.
 *
 * Los eventos salen del almacén local de vasak-accounts, por
 * `calendar.service.ts`, como el tablero de fecha y el centro de control: se
 * piden al montar, al cambiar de mes y cuando el sincronizador trae algo. Sin
 * servicio, sin permiso o con un error queda el mes solo.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { type IsoDate, type IsoMonth, MonthCalendar, monthOf } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { CALENDAR_CHANGED, calendarOccurrences } from '@/services/calendar.service';
import { localeTag, marksFromReply, weekStartFor } from '@/tools/calendar-sheet';
import { agendaFromReply, showsAgenda, todayIso, todayRange } from '@/tools/calendar-widget';
import { useBoxSize } from '@/tools/composables/useBoxSize';
import { gridRange } from '@/tools/date-board';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

const { t, locale } = useI18n();

const box = ref<HTMLElement | null>(null);
const { height } = useBoxSize(box);
const withAgenda = computed(() => showsAgenda(height.value));

const today = ref<IsoDate>(todayIso());
const month = ref<IsoMonth>(monthOf(today.value));
const language = computed(() => localeTag(locale.value));
const weekStart = computed(() => weekStartFor(locale.value));
const marked = ref<IsoDate[]>([]);
const agenda = ref(agendaFromReply(null));

/** El último pedido de cada tipo: una respuesta que llega tarde no pisa a la nueva. */
let marksRequest = 0;

async function loadMarks(): Promise<void> {
	const range = gridRange(month.value, weekStart.value);
	if (!range) return;
	const mine = ++marksRequest;
	try {
		const reply = await calendarOccurrences(range.from, range.to);
		if (mine === marksRequest) marked.value = marksFromReply(reply);
	} catch (error) {
		if (mine !== marksRequest) return;
		marked.value = [];
		logError('[widget-calendario] no se pudieron leer los eventos del mes:', error);
	}
}

async function loadAgenda(): Promise<void> {
	const { from, to } = todayRange(new Date());
	try {
		agenda.value = agendaFromReply(await calendarOccurrences(from, to));
	} catch (error) {
		agenda.value = [];
		logError('[widget-calendario] no se pudieron leer los eventos de hoy:', error);
	}
}

function showMonth(next: IsoMonth): void {
	if (next === month.value) return;
	month.value = next;
	marked.value = [];
	void loadMarks();
}

function timeOf(start: string): string {
	return new Date(start).toLocaleTimeString(language.value, { hour: '2-digit', minute: '2-digit' });
}

onMounted(() => {
	void loadMarks();
	void loadAgenda();
});
useSharedEvent(CALENDAR_CHANGED, () => {
	void loadMarks();
	void loadAgenda();
});
</script>

<template>
  <!-- Las medidas van contra la caja del marco: el mes llena lo que hay y la
       lista de hoy aparece sólo si sobra alto. -->
  <div ref="box" class="flex h-full w-full flex-col gap-[2cqmin] overflow-hidden p-[3cqmin]" data-calendar-widget>
    <MonthCalendar
      class="min-h-0 shrink-0"
      :month="month"
      :today="today"
      :marked-dates="marked"
      :locale="language"
      :week-start="weekStart"
      :previous-label="t('views.dateBoard.previousMonth')"
      :next-label="t('views.dateBoard.nextMonth')"
      :marked-label="t('views.dateBoard.hasEvents')"
      :today-label="t('views.dateBoard.today')"
      @update:month="showMonth"
    />
    <ul v-if="withAgenda" class="m-0 min-h-0 flex-1 list-none overflow-hidden p-0" data-calendar-agenda>
      <li v-if="agenda.length === 0" class="truncate text-[clamp(0.65rem,3.4cqmin,0.9rem)] text-tx-muted">
        {{ t('widgets.calendar.noEventsToday') }}
      </li>
      <li
        v-for="entry in agenda"
        :key="entry.occurrence_id"
        class="flex min-w-0 items-baseline gap-[2cqmin] text-[clamp(0.65rem,3.4cqmin,0.9rem)] text-tx-main"
      >
        <span class="shrink-0 tabular-nums text-tx-muted">{{ entry.all_day ? t('widgets.calendar.allDay') : timeOf(entry.start) }}</span>
        <span class="truncate">{{ entry.title }}</span>
      </li>
    </ul>
  </div>
</template>
