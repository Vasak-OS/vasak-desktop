<script setup lang="ts">
/**
 * El calendario del mes dentro del centro de control (vasak-desktop#183).
 *
 * Se abre tocando la fecha de la tarjeta de usuario y es una ficha con
 * «Volver», como la de Red o Bluetooth en los ajustes rápidos: lista → ficha.
 * No ocupa lugar fijo: la vista lo monta al abrirlo y lo desmonta al volver,
 * así que hoy se calcula cada vez que se abre y nada queda escuchando con el
 * calendario cerrado.
 *
 * Los días con eventos se piden **al abrir y al cambiar de mes**, nunca por
 * sondeo, al mismo almacén que lee el tablero de fecha. Si el servicio no
 * está, no hay permiso o falla, queda el mes solo.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	type IsoDate,
	type IsoMonth,
	MonthCalendar,
	monthOf,
	toIsoDate,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { calendarOccurrences } from '@/services/calendar.service';
import { localeTag, marksFromReply, weekStartFor } from '@/tools/calendar-sheet';
import { gridRange } from '@/tools/date-board';
import { logError } from '@/utils/logger';

const emit = defineEmits<{ back: [] }>();

const { t, locale } = useI18n();

const today = ref<IsoDate>(toIsoDate(new Date()));
const month = ref<IsoMonth>(monthOf(today.value));
const language = computed(() => localeTag(locale.value));
const weekStart = computed(() => weekStartFor(locale.value));
const marked = ref<IsoDate[]>([]);
/** El día tocado: sólo se resalta, el centro no tiene una lista de eventos que filtrar. */
const selected = ref<IsoDate | null>(null);

/** El último pedido: una respuesta de un mes anterior que llega tarde no pisa la del mes nuevo. */
let request = 0;

async function loadMarks(): Promise<void> {
	const range = gridRange(month.value, weekStart.value);
	if (!range) return;
	const mine = ++request;
	try {
		const reply = await calendarOccurrences(range.from, range.to);
		if (mine === request) marked.value = marksFromReply(reply);
	} catch (error) {
		if (mine !== request) return;
		marked.value = [];
		logError('[control-center] no se pudieron leer los eventos del mes:', error);
	}
}

function showMonth(next: IsoMonth): void {
	if (next === month.value) return;
	month.value = next;
	// Las marcas del mes anterior no corresponden a esta cuadrícula.
	marked.value = [];
	void loadMarks();
}

const backButton = ref<HTMLElement | null>(null);

onMounted(() => {
	// Lo primero de la ficha es «Volver»: ahí va el foco, como en los mosaicos.
	backButton.value?.querySelector('button')?.focus();
	void loadMarks();
});
</script>

<template>
  <section
    class="@container flex min-w-0 shrink-0 flex-col gap-2"
    :aria-label="t('views.controlCenter.calendar')"
    data-calendar-sheet
  >
    <div ref="backButton" class="flex shrink-0">
      <ActionButton
        :label="t('common.back')"
        icon="go-previous"
        icon-type="symbol"
        variant="ghost"
        size="sm"
        data-calendar-back
        @click="emit('back')"
      />
    </div>
    <div class="min-w-0 rounded-corner-l border border-ui-line bg-ui-surface/70 p-3">
      <MonthCalendar
        v-model="selected"
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
    </div>
  </section>
</template>
