<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	BarChart,
	type BarChartItem,
	CalendarHeatmap,
	EmptyState,
	ListRow,
	StatTile,
	SwitchToggle,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
import {
	clearScreenTime,
	getScreenTime,
	setScreenTimeEnabled,
} from '@/services/screen-time.service';
import {
	addDays,
	appsOf,
	type DurationWords,
	dailyAverage,
	dayOfMonth,
	dayTotal,
	differenceFromYesterday,
	formatDuration,
	isDay,
	monthOf,
	monthValues,
	rangeFor,
	type ScreenTimeRange,
	weekOf,
} from '@/tools/screen-time';
import { logError } from '@/utils/logger';

/**
 * El tablero de tiempo de pantalla (vasak-desktop#150), como el del video de
 * referencia (segundo 14): «‹ Hoy ›» arriba, el promedio de la semana, el
 * total del día en grande y la diferencia con ayer; la semana en barras con
 * el día que se mira en el acento y el mes en un mapa de calor; y la lista de
 * aplicaciones con una barra proporcional.
 *
 * Lo abre el botón del centro de control. Los datos los guarda el backend
 * (`src-tauri/src/screen_time/`) y las cuentas son de `tools/screen-time.ts`;
 * acá sólo se dibuja. Las piezas son de vue-libvasak (`StatTile`, `BarChart`,
 * `CalendarHeatmap`, `ListRow` con barra): nada dibujado a mano.
 *
 * # Una columna por vez en angosto
 *
 * En el ancho de siempre (580) los tres bloques van en fila y la semana al
 * lado del mes. Por debajo de 30 rem, por consulta de contenedor sobre la
 * tarjeta, todo se apila en una sola columna que se desplaza: nada se corta
 * ni se pisa a 240 o 360.
 *
 * # Privacidad
 *
 * Abajo, el interruptor que apaga el registro (`screen_time.enabled` en
 * `vasak.conf`) y «Borrar historial», que pide confirmación en el lugar.
 * Nada sale de la computadora; lo dice la misma línea.
 */

const { t, locale } = useI18n();

const range = ref<ScreenTimeRange | null>(null);
const selected = ref<string | null>(null);
const failed = ref(false);
const confirming = ref(false);
const busy = ref(false);
let timer: ReturnType<typeof setInterval> | undefined;

const words = computed<DurationWords>(() => ({
	hoursMinutes: t('views.screenTimeApplet.duration.hoursMinutes'),
	hours: t('views.screenTimeApplet.duration.hours'),
	minutes: t('views.screenTimeApplet.duration.minutes'),
	underMinute: t('views.screenTimeApplet.duration.underMinute'),
}));
const duration = (ms: number) => formatDuration(ms, words.value);

const today = computed(() => range.value?.today ?? null);
const day = computed(() => selected.value ?? today.value);
const days = computed(() => range.value?.days ?? {});
const firstDay = computed(() => range.value?.first_day ?? null);
const enabled = computed(() => range.value?.enabled ?? true);

const load = async (target?: string | null) => {
	const wanted = target ?? day.value ?? new Date().toISOString().slice(0, 10);
	const { from, to } = rangeFor(isDay(wanted) ? wanted : new Date().toISOString().slice(0, 10));
	try {
		let response = await getScreenTime(from, to);
		// La primera vez no se sabe qué día es en la sesión (la fecha de la
		// página es la de UTC): si el de verdad cae en otro rango, se pide ése.
		if (!target && isDay(response.today)) {
			const real = rangeFor(response.today);
			if (real.from !== from || real.to !== to) response = await getScreenTime(real.from, real.to);
		}
		range.value = response;
		failed.value = false;
		if (target) selected.value = target;
	} catch (error) {
		failed.value = true;
		logError('[ScreenTime] no se pudo leer el historial:', error);
	}
};

/** Al volver a abrirlo, otra vez en hoy. */
const onShown = () => {
	selected.value = null;
	confirming.value = false;
	void load(null);
	start();
};

const start = () => {
	stop();
	// Mientras se ve, el día de hoy sigue sumando.
	timer = setInterval(() => {
		if (!selected.value || selected.value === today.value) void load(null);
	}, 30_000);
};
const stop = () => {
	if (timer !== undefined) clearInterval(timer);
	timer = undefined;
};

onMounted(() => {
	void load(null);
	start();
});
onBeforeUnmount(stop);

const canGoBack = computed(() =>
	Boolean(day.value && (!firstDay.value || day.value > firstDay.value))
);
const canGoForward = computed(() => Boolean(day.value && today.value && day.value < today.value));
const go = (amount: number) => {
	if (!day.value) return;
	void load(addDays(day.value, amount));
};

const dateFormat = computed(
	() =>
		new Intl.DateTimeFormat(locale.value || undefined, {
			weekday: 'short',
			day: 'numeric',
			month: 'short',
			timeZone: 'UTC',
		})
);
const shortDate = computed(
	() =>
		new Intl.DateTimeFormat(locale.value || undefined, {
			day: 'numeric',
			month: 'short',
			timeZone: 'UTC',
		})
);
const asDate = (text: string) => new Date(`${text}T12:00:00Z`);

const title = computed(() => {
	if (!day.value || !today.value) return t('views.screenTimeApplet.today');
	if (day.value === today.value) return t('views.screenTimeApplet.today');
	if (day.value === addDays(today.value, -1)) return t('views.screenTimeApplet.yesterday');
	const text = dateFormat.value.format(asDate(day.value));
	return text.charAt(0).toLocaleUpperCase() + text.slice(1);
});

const week = computed(() => (day.value ? weekOf(day.value) : []));
const weekRange = computed(() => {
	const [first, last] = [week.value[0], week.value[6]];
	return first && last
		? `${shortDate.value.format(asDate(first))} – ${shortDate.value.format(asDate(last))}`
		: '';
});

const total = computed(() => (day.value ? dayTotal(days.value, day.value) : 0));
const average = computed(() =>
	day.value && today.value ? dailyAverage(days.value, day.value, today.value, firstDay.value) : null
);
const difference = computed(() =>
	day.value ? differenceFromYesterday(days.value, day.value, firstDay.value) : null
);
const differenceText = computed(() => {
	const value = difference.value;
	if (value === null) return t('views.screenTimeApplet.noYesterday');
	if (Math.abs(value) < 60_000) return t('views.screenTimeApplet.sameAsYesterday');
	const amount = duration(Math.abs(value));
	const said = t(value > 0 ? 'views.screenTimeApplet.more' : 'views.screenTimeApplet.less').replace(
		'{0}',
		amount
	);
	return t('views.screenTimeApplet.vsYesterday').replace('{0}', said);
});
const differenceIcon = computed(() => {
	const value = difference.value;
	if (value === null || Math.abs(value) < 60_000) return null;
	return value > 0 ? 'go-up' : 'go-down';
});

const weekdayLong = computed(
	() => new Intl.DateTimeFormat(locale.value || undefined, { weekday: 'short', timeZone: 'UTC' })
);
const weekdayNarrow = computed(
	() => new Intl.DateTimeFormat(locale.value || undefined, { weekday: 'narrow', timeZone: 'UTC' })
);
const bars = computed<BarChartItem[]>(() =>
	week.value.map((each) => {
		const value = dayTotal(days.value, each);
		const name = weekdayLong.value.format(asDate(each)).replace('.', '');
		return {
			key: each,
			label: name.charAt(0).toLocaleUpperCase() + name.slice(1),
			shortLabel: weekdayNarrow.value.format(asDate(each)).toLocaleUpperCase(),
			value,
			valueLabel: duration(value),
			current: each === day.value,
		};
	})
);

const month = computed(() => (day.value ? monthOf(day.value) : null));
const heat = computed(() => (day.value ? monthValues(days.value, day.value) : {}));
const todayInMonth = computed(() =>
	today.value && month.value && today.value >= month.value.first && today.value <= month.value.last
		? dayOfMonth(today.value)
		: undefined
);

const apps = computed(() => (range.value && day.value ? appsOf(range.value, day.value) : []));
const nothing = computed(() => apps.value.length === 0);

const toggle = async (value: boolean) => {
	busy.value = true;
	try {
		await setScreenTimeEnabled(value);
		if (range.value) range.value = { ...range.value, enabled: value };
	} catch (error) {
		logError('[ScreenTime] no se pudo cambiar el registro:', error);
	} finally {
		busy.value = false;
	}
};

const clearAll = async () => {
	busy.value = true;
	try {
		await clearScreenTime();
		confirming.value = false;
		selected.value = null;
		await load(null);
	} catch (error) {
		logError('[ScreenTime] no se pudo borrar el historial:', error);
	} finally {
		busy.value = false;
	}
};
</script>

<template>
  <AppletPopover applet="screen-time" :aria-label="t('views.screenTimeApplet.title')" @shown="onShown" @leave="stop">
    <div class="@container flex h-full min-h-0 flex-col">
      <div class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto">
        <!-- ‹ Hoy › -->
        <header class="flex min-w-0 shrink-0 items-center justify-between gap-2">
          <ActionButton
            label=""
            icon="go-previous"
            :icon-alt="t('views.screenTimeApplet.previousDay')"
            :title="t('views.screenTimeApplet.previousDay')"
            variant="ghost"
            :disabled="!canGoBack"
            @click="go(-1)" />
          <h2 class="m-0 min-w-0 truncate text-center text-heading-s font-semibold text-tx-main" aria-live="polite">
            {{ title }}
          </h2>
          <ActionButton
            label=""
            icon="go-next"
            :icon-alt="t('views.screenTimeApplet.nextDay')"
            :title="t('views.screenTimeApplet.nextDay')"
            variant="ghost"
            :disabled="!canGoForward"
            @click="go(1)" />
        </header>

        <!-- Promedio, total y diferencia con ayer. -->
        <section class="grid shrink-0 grid-cols-1 gap-2 @[30rem]:grid-cols-3" data-screen-time-stats>
          <StatTile
            :label="t('views.screenTimeApplet.dailyAverage')"
            :value="average === null ? t('views.screenTimeApplet.noAverage') : duration(average)"
            :hint="weekRange" />
          <div class="flex min-w-0 flex-col items-center justify-center gap-1 rounded-corner-l border border-ui-line bg-ui-surface/70 p-3">
            <p class="m-0 truncate text-label-s text-tx-muted">{{ t('views.screenTimeApplet.total') }}</p>
            <p class="m-0 max-w-full truncate text-heading-l font-semibold tabular-nums text-tx-main" :title="duration(total)">
              {{ duration(total) }}
            </p>
          </div>
          <div class="flex min-w-0 items-center gap-2 rounded-corner-l border border-ui-line bg-ui-surface/70 p-3">
            <ThemeIcon v-if="differenceIcon" :name="differenceIcon" type="symbol" :size="16" />
            <p class="m-0 min-w-0 break-words text-label-m text-tx-main">{{ differenceText }}</p>
          </div>
        </section>

        <!-- La semana y el mes. -->
        <section class="flex shrink-0 flex-col gap-2 @[30rem]:flex-row">
          <div class="flex h-36 min-w-0 flex-col rounded-corner-l border border-ui-line bg-ui-surface/70 p-3 @[30rem]:h-auto @[30rem]:flex-1">
            <BarChart :bars="bars" :label="t('views.screenTimeApplet.weekOf').replace('{0}', weekRange)" />
          </div>
          <div class="min-w-0 rounded-corner-l border border-ui-line bg-ui-surface/70 p-3 @[30rem]:w-40 @[30rem]:shrink-0">
            <CalendarHeatmap
              v-if="month"
              :year="month.year"
              :month="month.month"
              :values="heat"
              :today="todayInMonth"
              :selected="day ? dayOfMonth(day) : undefined"
              :locale="locale || undefined"
              :format-value="duration" />
          </div>
        </section>

        <!-- Las aplicaciones. -->
        <section class="flex shrink-0 flex-col gap-1 rounded-corner-l border border-ui-line bg-ui-surface/70 p-1" :aria-label="t('views.screenTimeApplet.apps')">
          <EmptyState
            v-if="failed"
            size="sm"
            icon="dialog-warning"
            icon-type="symbol"
            :title="t('views.screenTimeApplet.loadError')" />
          <div v-else-if="!enabled && nothing" data-screen-time-disabled>
            <EmptyState
              size="sm"
              icon="media-playback-pause"
              icon-type="symbol"
              :title="t('views.screenTimeApplet.disabled')"
              :note="t('views.screenTimeApplet.disabledNote')" />
          </div>
          <div v-else-if="nothing" data-screen-time-empty>
            <EmptyState
              size="sm"
              icon="preferences-system-time"
              icon-type="symbol"
              :title="t('views.screenTimeApplet.empty')"
              :note="t('views.screenTimeApplet.emptyNote')" />
          </div>
          <ListRow
            v-for="app in apps"
            v-else
            :key="app.appId"
            :title="app.name"
            :icon="app.icon"
            :meta="duration(app.ms)"
            :bar="app.share"
            truncate
            hoverable />
        </section>
      </div>

      <!-- Privacidad: apagar y borrar, sin salir del tablero. -->
      <footer class="flex shrink-0 flex-wrap items-center justify-between gap-2 border-t border-ui-line-weak pt-3">
        <div class="flex min-w-0 items-center gap-2 text-label-s text-tx-main">
          <SwitchToggle
            :model-value="enabled"
            :label="t('views.screenTimeApplet.record')"
            :disabled="busy"
            @update:model-value="toggle" />
          <span class="flex min-w-0 flex-col">
            <span class="break-words">{{ t('views.screenTimeApplet.record') }}</span>
            <span class="break-words text-body-xs text-tx-muted">{{ t('views.screenTimeApplet.privacy') }}</span>
          </span>
        </div>
        <fieldset v-if="confirming" class="m-0 flex min-w-0 flex-wrap items-center gap-2 border-0 p-0">
          <legend class="sr-only">{{ t('views.screenTimeApplet.clearQuestion') }}</legend>
          <ActionButton :label="t('views.screenTimeApplet.cancel')" variant="ghost" size="sm" @click="confirming = false" />
          <ActionButton :label="t('views.screenTimeApplet.confirmClear')" variant="danger" size="sm" :loading="busy" @click="clearAll" />
        </fieldset>
        <ActionButton
          v-else
          :label="t('views.screenTimeApplet.clear')"
          icon="edit-clear-history"
          variant="ghost"
          size="sm"
          :title="t('views.screenTimeApplet.clearQuestion')"
          @click="confirming = true" />
      </footer>
    </div>
  </AppletPopover>
</template>
