<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	addDays,
	type CalendarEntry,
	ClockDisplay,
	EmptyState,
	EventList,
	entriesOn,
	HourlyForecast,
	type IsoDate,
	type IsoMonth,
	LoadingState,
	MonthCalendar,
	markedDates,
	monthOf,
	ProgressRing,
	parseIsoDate,
	ThemeIcon,
	toIsoDate,
	weekStartOf,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
import {
	type CalendarOccurrence,
	type CalendarReply,
	calendarLocations,
	calendarOccurrences,
	openCalendar,
} from '@/services/calendar.service';
import { useWeather } from '@/tools/composables/useWeather';
import {
	type DayWeather,
	dayWeather,
	type ForecastData,
	gridRange,
	occurrenceKey,
	relativeDayLabel,
	toCalendarEntry,
	upcomingHours,
	weatherIcon,
	weatherRings,
} from '@/tools/date-board';
import { logError } from '@/utils/logger';

/**
 * El tablero de fecha que cuelga del reloj del panel (vasak-desktop#130).
 *
 * Es el §3 de la referencia del video: el mes a la izquierda, el reloj grande
 * con el clima de las próximas horas en arco en el centro, el clima del día con
 * sus cuatro anillos a la derecha, y abajo, separados por una línea fina, los
 * eventos de un día. Todo es de vue-libvasak; acá sólo se juntan las dos
 * fuentes que ya tiene el escritorio:
 *
 * - **el clima** de `useWeather`, el mismo del widget, del panel y del menú, con
 *   el pedido ampliado a horas, viento, humedad y sensación;
 * - **los eventos** del almacén local de vasak-accounts (`calendar.service.ts`),
 *   el servicio que mantiene la copia de los calendarios y contesta desde lo
 *   guardado (decisión 6). Si no está, o no hay permiso, el bloque lo dice con
 *   un vacío: nunca roto, nunca inventado.
 *
 * # Qué día muestra cada bloque
 *
 * Como en el video, al abrir el clima es el de **hoy** y los eventos los de
 * **mañana** —lo que se mira a la noche es qué hay mañana—. Elegir un día en el
 * calendario, o moverse con ‹ › en el clima, lleva **los dos** a ese día. Cada
 * vez que se abre vuelve a empezar así: el tablero contesta «qué día es y qué
 * tengo», y un día elegido ayer no es la respuesta.
 *
 * # En una ventana angosta
 *
 * El applet mide 960 × 540. Si el monitor no da para eso, el backend lo achica,
 * y por debajo de 48 rem de ancho (`@3xl`, por contenedor) las tres columnas se
 * apilan y el tablero se desplaza: una columna por vez, sin cortar nada.
 */

const { t } = useI18n();

// ── El día ───────────────────────────────────────────────────────────────────

const now = ref(new Date());
const today = computed<IsoDate>(() => toIsoDate(now.value));
/** El día elegido. `null` es el de al abrir: el clima de hoy y los eventos de mañana. */
const selected = ref<IsoDate | null>(null);
const weatherDay = computed<IsoDate>(() => selected.value ?? today.value);
const eventsDay = computed<IsoDate>(() => selected.value ?? addDays(today.value, 1));
const visibleMonth = ref<IsoMonth>(monthOf(today.value));
const weekStart = weekStartOf();

let clock: ReturnType<typeof setInterval> | undefined;
onMounted(() => {
	// Una vez por minuto alcanza para que hoy, la hora del arco y el evento en
	// curso no se queden viejos; el reloj grande tiene su propio temporizador.
	clock = setInterval(() => {
		now.value = new Date();
	}, 60_000);
});
onUnmounted(() => {
	if (clock) clearInterval(clock);
});

function pick(date: IsoDate): void {
	selected.value = date;
	if (monthOf(date) !== visibleMonth.value) visibleMonth.value = monthOf(date);
}

function stepDay(amount: number): void {
	pick(addDays(weatherDay.value, amount));
}

const longDate = (date: IsoDate, withWeekday = true) =>
	parseIsoDate(date)?.toLocaleDateString(undefined, {
		weekday: withWeekday ? 'long' : undefined,
		day: 'numeric',
		month: 'long',
	}) ?? date;

const weekday = (date: IsoDate) =>
	parseIsoDate(date)?.toLocaleDateString(undefined, { weekday: 'long' }) ?? date;

// ── El clima ─────────────────────────────────────────────────────────────────

const { weather, failed: weatherFailed, loading: weatherLoading } = useWeather();
const forecast = computed(() => weather.value as ForecastData | null);

const describe = (code: number | null | undefined) => {
	if (code === null || code === undefined) return '';
	const key = `views.dateBoard.conditions.${code}`;
	const text = t(key);
	return text === key ? '' : text;
};

const arc = computed(() => upcomingHours(forecast.value, now.value, describe));
const day = computed<DayWeather | null>(() =>
	dayWeather(forecast.value, weatherDay.value, now.value)
);

const degrees = new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 });
const tempText = (value: number | null) => (value === null ? '–' : `${degrees.format(value)}°`);

/** Los cuatro anillos del día, los mismos que el widget de clima. */
const rings = computed(() =>
	weatherRings(day.value, {
		wind: t('views.dateBoard.wind'),
		humidity: t('views.dateBoard.humidity'),
		rain: t('views.dateBoard.rain'),
		feelsLike: t('views.dateBoard.feelsLike'),
	})
);

// ── Los eventos ──────────────────────────────────────────────────────────────

const reply = ref<CalendarReply | null>(null);
const loadingEvents = ref(false);
const locations = ref<Record<string, string>>({});
/** El rango pedido por última vez: una respuesta vieja que llega tarde no pisa la nueva. */
let request = 0;

const occurrences = computed<CalendarOccurrence[]>(() =>
	reply.value?.state === 'ready' ? reply.value.entries : []
);
const entries = computed<CalendarEntry[]>(() =>
	occurrences.value.map((occurrence) =>
		toCalendarEntry(occurrence, locations.value[occurrenceKey(occurrence)])
	)
);
const marked = computed(() => markedDates(entries.value));
const dayEntries = computed(() => entriesOn(entries.value, eventsDay.value));

async function loadEvents(): Promise<void> {
	const range = gridRange(visibleMonth.value, weekStart);
	if (!range) return;
	const mine = ++request;
	loadingEvents.value = reply.value === null;
	try {
		const answer = await calendarOccurrences(range.from, range.to);
		if (mine !== request) return;
		reply.value = answer;
	} catch (error) {
		if (mine !== request) return;
		logError('[DateBoard] no se pudieron leer los eventos:', error);
		reply.value = { state: 'failed', detail: String(error) };
	} finally {
		if (mine === request) loadingEvents.value = false;
	}
}

/** El lugar sólo de los eventos del día que se ve: cada uno es una llamada. */
async function loadLocations(): Promise<void> {
	const byKey = new Map(
		occurrences.value.map((occurrence) => [occurrenceKey(occurrence), occurrence])
	);
	const missing = dayEntries.value
		.map((entry) => byKey.get(entry.id))
		.filter((occurrence): occurrence is CalendarOccurrence => Boolean(occurrence))
		.filter((occurrence) => !(occurrenceKey(occurrence) in locations.value));
	if (missing.length === 0) return;
	try {
		const found = await calendarLocations(missing);
		const next = { ...locations.value };
		missing.forEach((occurrence, index) => {
			next[occurrenceKey(occurrence)] = found[index] ?? '';
		});
		locations.value = next;
	} catch (error) {
		logError('[DateBoard] no se pudieron leer los lugares:', error);
	}
}

watch(visibleMonth, () => void loadEvents());
watch(dayEntries, () => void loadLocations());

const eventsHeading = computed(() => {
	const relative = relativeDayLabel(eventsDay.value, today.value);
	const text = longDate(eventsDay.value);
	return relative ? `${text} · ${relative}` : text;
});

async function launchCalendar(): Promise<void> {
	try {
		await openCalendar();
	} catch (error) {
		logError('[DateBoard] no se pudo abrir el calendario:', error);
	}
}

/**
 * Cada vez que vuelve a la vista.
 *
 * Esconder el applet no destruye el webview: lo que cambió mientras estaba
 * escondido —la hora, los eventos— se vuelve a pedir acá, y el día elegido
 * vuelve a ser el de al abrir.
 */
function onShown(): void {
	now.value = new Date();
	selected.value = null;
	const month = monthOf(today.value);
	if (visibleMonth.value !== month) visibleMonth.value = month;
	else void loadEvents();
}

onMounted(() => void loadEvents());
</script>

<template>
  <AppletPopover applet="date" :aria-label="t('views.dateBoard.title')" @shown="onShown">
    <div class="date-board @container relative h-full min-h-0 overflow-x-hidden overflow-y-auto" data-date-board>
      <!-- El círculo de fondo del video: un aro finísimo detrás del reloj. -->
      <span aria-hidden="true" class="date-board-circle pointer-events-none absolute rounded-corner-full border border-ui-line-weak"></span>

      <div class="relative flex min-h-full min-w-0 flex-col gap-4">
        <div class="grid min-w-0 grid-cols-1 gap-4 @3xl:grid-cols-[15.5rem_minmax(0,1fr)_18rem]">
          <!-- El mes. -->
          <section
            class="min-w-0 rounded-corner-l border border-ui-line bg-ui-surface/70 p-3"
            data-date-board-calendar
            :aria-label="t('views.dateBoard.calendar')">
            <MonthCalendar
              :model-value="selected"
              :month="visibleMonth"
              :today="today"
              :marked-dates="marked"
              :week-start="weekStart"
              :previous-label="t('views.dateBoard.previousMonth')"
              :next-label="t('views.dateBoard.nextMonth')"
              :marked-label="t('views.dateBoard.hasEvents')"
              :today-label="t('views.dateBoard.today')"
              @update:month="visibleMonth = $event"
              @select="pick">
              <template #actions>
                <!-- En el ancho de un teléfono el «+» le deja lugar al nombre del
                     mes: lo mismo hace «Abrir en Calendario», abajo. Se mide
                     contra el calendario, que es el contenedor más cercano. -->
                <span class="hidden @min-[13rem]:contents">
                  <ActionButton
                    label=""
                    icon="list-add"
                    :icon-alt="t('views.dateBoard.newEvent')"
                    :title="t('views.dateBoard.newEvent')"
                    variant="ghost"
                    size="sm"
                    @click="launchCalendar" />
                </span>
              </template>
            </MonthCalendar>
          </section>

          <!-- El reloj grande, con las próximas horas en arco. -->
          <section class="flex min-w-0 flex-col items-center justify-center px-6 @3xl:px-0" :aria-label="t('views.dateBoard.now')">
            <HourlyForecast
              :hours="arc.hours"
              :current="arc.current"
              :hour12="false"
              :label="t('views.dateBoard.hourly')">
              <ClockDisplay size="lg" seconds small-seconds :hour12="false" />
            </HourlyForecast>
          </section>

          <!-- El clima del día. -->
          <section
            class="flex min-w-0 flex-col gap-3 rounded-corner-l border border-ui-line bg-ui-surface/70 p-3"
            :aria-label="t('views.dateBoard.weather')">
            <div class="flex min-w-0 items-center gap-1">
              <ActionButton
                label=""
                icon="go-previous"
                :icon-alt="t('views.dateBoard.previousDay')"
                :title="t('views.dateBoard.previousDay')"
                variant="ghost"
                size="sm"
                @click="stepDay(-1)" />
              <h2 class="m-0 min-w-0 flex-1 truncate text-center text-label-s font-semibold uppercase tracking-wide text-tx-main" aria-live="polite">
                {{ weekday(weatherDay) }}
              </h2>
              <ActionButton
                label=""
                icon="go-next"
                :icon-alt="t('views.dateBoard.nextDay')"
                :title="t('views.dateBoard.nextDay')"
                variant="ghost"
                size="sm"
                @click="stepDay(1)" />
            </div>

            <template v-if="day">
              <div class="flex min-w-0 items-center gap-3">
                <ThemeIcon :name="weatherIcon(day.code, day.isDay)" :size="48" class="shrink-0" />
                <div class="flex min-w-0 flex-col">
                  <p class="m-0 truncate text-display-m font-light text-tx-main tabular-nums" data-day-temperature>
                    {{ tempText(day.temperature) }}
                  </p>
                  <p class="m-0 truncate text-label-m font-medium text-tx-main">{{ describe(day.code) }}</p>
                  <p class="m-0 truncate text-label-xs text-tx-muted tabular-nums">
                    {{ t('views.dateBoard.max').replace('{0}', () => tempText(day?.max ?? null)) }}
                    ·
                    {{ t('views.dateBoard.min').replace('{0}', () => tempText(day?.min ?? null)) }}
                  </p>
                </div>
              </div>
              <div class="grid min-w-0 grid-cols-2 gap-2 @sm:grid-cols-4" data-weather-rings>
                <ProgressRing
                  v-for="ring in rings"
                  :key="ring.key"
                  :value="ring.value"
                  :label="ring.label"
                  :display="ring.display"
                  :unit="ring.unit"
                  size="sm" />
              </div>
            </template>
            <LoadingState v-else-if="weatherLoading" :label="t('views.dateBoard.weatherLoading')" size="sm" />
            <EmptyState
              v-else
              :title="weatherFailed ? t('views.dateBoard.weatherFailed') : t('views.dateBoard.noForecast')"
              icon=""
              size="sm"
              muted />
          </section>
        </div>

        <!-- El piso de abajo: los eventos de un día. -->
        <section class="relative flex min-w-0 flex-col gap-3 border-t border-ui-line pt-3" :aria-label="t('views.dateBoard.events')">
          <span aria-hidden="true" class="date-board-wave pointer-events-none absolute inset-x-0 bottom-0 h-16"></span>
          <div class="relative flex min-w-0 flex-wrap items-center gap-2">
            <ThemeIcon name="x-office-calendar" type="symbol" :size="16" class="shrink-0" />
            <h2 class="m-0 min-w-40 flex-1 break-words text-label-m font-semibold text-tx-main first-letter:uppercase" data-events-heading>
              {{ eventsHeading }}
            </h2>
            <ActionButton
              :label="t('views.dateBoard.openCalendar')"
              icon="window-new"
              icon-right
              variant="secondary"
              size="sm"
              @click="launchCalendar" />
          </div>

          <div class="relative min-w-0">
            <LoadingState v-if="loadingEvents" :label="t('views.dateBoard.loadingEvents')" size="sm" />
            <EmptyState
              v-else-if="reply?.state === 'unavailable'"
              :title="t('views.dateBoard.unavailable')"
              :note="t('views.dateBoard.unavailableNote')"
              icon=""
              size="sm"
              muted />
            <EmptyState
              v-else-if="reply?.state === 'denied'"
              :title="t('views.dateBoard.denied')"
              :note="t('views.dateBoard.deniedNote')"
              icon=""
              size="sm"
              muted />
            <EmptyState
              v-else-if="reply?.state === 'failed'"
              :title="t('views.dateBoard.failed')"
              icon=""
              size="sm"
              muted />
            <template v-else>
              <EventList
                :entries="dayEntries"
                :now="now"
                :hour12="false"
                :empty-label="t('views.dateBoard.noEvents')"
                :all-day-label="t('views.dateBoard.allDay')"
                :ongoing-label="t('views.dateBoard.ongoing')" />
              <p v-if="reply?.state === 'ready' && reply.truncated" class="m-0 mt-2 text-label-xs text-tx-muted">
                {{ t('views.dateBoard.truncated') }}
              </p>
            </template>
          </div>
        </section>
      </div>
    </div>
  </AppletPopover>
</template>

<style scoped>
/* El círculo decorativo: centrado en la columna del reloj y más grande que
   ella, como en el video. Sólo el aro, en el canto más tenue del esquema. */
.date-board-circle {
  width: 30rem;
  height: 30rem;
  left: 50%;
  top: 8rem;
  transform: translate(-50%, -50%);
}

/* La curva ondulada tenue del piso de abajo: medio círculos encadenados en el
   acento casi transparente. Va detrás de las tarjetas. */
.date-board-wave {
  background-image: radial-gradient(
    circle at 50% 100%,
    transparent 58%,
    color-mix(in srgb, var(--use-primary) 14%, transparent) 60%,
    color-mix(in srgb, var(--use-primary) 14%, transparent) 62%,
    transparent 64%
  );
  background-size: 6rem 3rem;
  background-repeat: repeat-x;
  background-position: 0 100%;
  opacity: 0.8;
}
</style>
