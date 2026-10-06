<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	type IsoDate,
	ProgressRing,
	parseIsoDate,
	SegmentedControl,
	type SegmentedOption,
	ThemeIcon,
	toIsoDate,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useWeather } from '@/tools/composables/useWeather';
import { dayWeather, type ForecastData, ringFill, weatherIcon } from '@/tools/date-board';

/**
 * El widget de clima del escritorio, con la estética del clima del tablero de
 * fecha (vasak-desktop#167).
 *
 * Muestra el clima de un día como el applet de fecha (`DateBoardAppletView`): el
 * icono y la temperatura grande, la condición, la máxima y la mínima, y los
 * cuatro anillos —viento, humedad, lluvia y sensación— de `ProgressRing`. Para
 * ver otros días, las flechas ‹ › y una fila de tarjetas pasantes abajo, como en
 * el tablero. Reusa las mismas cuentas que el tablero (`date-board.ts`:
 * `dayWeather`, `ringFill`, `weatherIcon`) sobre el clima de `useWeather`, el
 * mismo de todo el escritorio: no hay un segundo pedido.
 *
 * Las dos variantes se ven igual; la diferencia es el tamaño por omisión y que
 * `today` arranca —y se queda— en hoy. Lo que entra lo decide el tamaño medido
 * de la celda (el WebView no avisa de `resize`, así que va un `ResizeObserver`):
 * los anillos aparecen con alto, y las tarjetas pasantes con más alto todavía.
 */

const { t } = useI18n();

const props = defineProps<{
	/** `extended` es el pronóstico de la semana; `today`, el día de hoy. */
	variant?: string;
}>();

const soloHoy = computed(() => props.variant === 'today');

// Los datos no son de este componente: los pide una sola ventana y los comparte
// el cache de Rust. Ver useWeather.
const { weather, failed } = useWeather();
const forecast = computed(() => weather.value as ForecastData | null);

// La hora, para que «hoy» y el clima de ahora no se queden viejos.
const now = ref(new Date());
let clock: ReturnType<typeof setInterval> | undefined;
onMounted(() => {
	clock = setInterval(() => {
		now.value = new Date();
	}, 60_000);
});
onUnmounted(() => {
	if (clock) clearInterval(clock);
});

const today = computed<IsoDate>(() => toIsoDate(now.value));
/** El día elegido; `null` es hoy. En la variante de hoy no se mueve. */
const selected = ref<IsoDate | null>(null);
const weatherDay = computed<IsoDate>(() => selected.value ?? today.value);
const day = computed(() => dayWeather(forecast.value, weatherDay.value, now.value));

/** Los días del pronóstico, para las flechas y las tarjetas pasantes. */
const days = computed<IsoDate[]>(() => (forecast.value?.daily?.time ?? []) as IsoDate[]);

function pick(date: IsoDate): void {
	if (soloHoy.value) return;
	selected.value = date;
}
function stepDay(amount: number): void {
	if (soloHoy.value) return;
	const list = days.value;
	const index = list.indexOf(weatherDay.value);
	if (index < 0) return;
	const next = list[Math.min(list.length - 1, Math.max(0, index + amount))];
	if (next) selected.value = next;
}
const canPrev = computed(() => !soloHoy.value && days.value.indexOf(weatherDay.value) > 0);
const canNext = computed(() => {
	if (soloHoy.value) return false;
	const index = days.value.indexOf(weatherDay.value);
	return index >= 0 && index < days.value.length - 1;
});

const whole = new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 });
const tempText = (value: number | null) => (value === null ? '–' : `${whole.format(value)}°`);
const weekdayName = (date: IsoDate) =>
	parseIsoDate(date)?.toLocaleDateString(undefined, { weekday: 'long' }) ?? date;
const shortWeekday = (date: IsoDate) =>
	parseIsoDate(date)?.toLocaleDateString(undefined, { weekday: 'short' }) ?? date;

/** Los días como opciones del control segmentado: el nombre corto y la máxima. */
const dayOptions = computed<SegmentedOption<IsoDate>[]>(() =>
	days.value.map((date, index) => ({
		value: date,
		label: shortWeekday(date),
		badge: tempText(forecast.value?.daily?.temperature_2m_max?.[index] ?? null),
	}))
);

const describe = (code: number | null | undefined) => {
	if (code === null || code === undefined) return '';
	const key = `views.dateBoard.conditions.${code}`;
	const text = t(key);
	return text === key ? '' : text;
};

/** Los cuatro anillos del día, como en el tablero de fecha. */
const rings = computed(() => {
	const d = day.value;
	if (!d) return [];
	const n = (value: number | null) => (value === null ? '–' : whole.format(value));
	return [
		{
			key: 'wind',
			label: t('components.WeatherWidget.wind'),
			display: n(d.wind),
			unit: d.units.wind,
			value: ringFill('wind', d.wind, d.units.wind),
		},
		{
			key: 'humidity',
			label: t('components.WeatherWidget.humidity'),
			display: d.humidity === null ? '–' : `${n(d.humidity)}%`,
			unit: undefined,
			value: ringFill('humidity', d.humidity),
		},
		{
			key: 'rain',
			label: t('components.WeatherWidget.rain'),
			display: d.rain === null ? '–' : `${n(d.rain)}%`,
			unit: undefined,
			value: ringFill('rain', d.rain),
		},
		{
			key: 'feelsLike',
			label: t('components.WeatherWidget.feelsLike'),
			display: d.feelsLike === null ? '–' : `${n(d.feelsLike)}°`,
			unit: undefined,
			value: ringFill('feelsLike', d.feelsLike, d.units.temperature),
		},
	];
});

/** Hay con qué dibujar cuando el día del pronóstico está. */
const listo = computed(() => day.value !== null);

// ── El tamaño medido ───────────────────────────────────────────────────────────

const box = ref<HTMLElement | null>(null);
const boxWidth = ref(0);
const boxHeight = ref(0);
let observer: ResizeObserver | null = null;
onMounted(() => {
	if (!box.value) return;
	observer = new ResizeObserver(() => {
		boxWidth.value = box.value?.clientWidth ?? 0;
		boxHeight.value = box.value?.clientHeight ?? 0;
	});
	observer.observe(box.value);
	boxWidth.value = box.value.clientWidth;
	boxHeight.value = box.value.clientHeight;
});
onUnmounted(() => observer?.disconnect());

/** Los anillos entran cuando la celda tiene alto; las tarjetas, con más. */
const showRings = computed(() => boxHeight.value >= 260);
const showStrip = computed(() => !soloHoy.value && boxHeight.value >= 400 && days.value.length > 1);
/** En celda angosta, los anillos van de a dos; con ancho, los cuatro en fila. */
const ringColumns = computed(() => (boxWidth.value >= 260 ? 4 : 2));
/** En celda muy baja y ancha —la variante de hoy— el resumen va en fila. */
const compactRow = computed(() => boxHeight.value < 150);
</script>

<template>
  <div ref="box" class="h-full min-h-0 w-full p-[3cqmin]">
    <!-- Sin datos todavía, o sin red: el widget dice qué pasa en vez de quedar
         en blanco. -->
    <div
      v-if="!listo"
      class="flex h-full items-center justify-center p-2 text-center text-tx-main/60"
    >
      {{ failed ? t('components.WeatherWidget.failed') : t('components.WeatherWidget.loading') }}
    </div>

    <!-- El clima del día, en una tarjeta como la del tablero de fecha. -->
    <div
      v-else
      class="flex h-full min-h-0 flex-col gap-[3cqmin] rounded-corner-l border border-ui-line bg-ui-surface/70 p-[3cqmin]"
    >
      <!-- Qué día: con flechas para pasar a otro, salvo en la variante de hoy. -->
      <div class="flex min-w-0 items-center gap-1">
        <ActionButton
          v-if="!soloHoy"
          label=""
          icon="go-previous"
          :icon-alt="t('components.WeatherWidget.previousDay')"
          :title="t('components.WeatherWidget.previousDay')"
          variant="ghost"
          size="sm"
          :disabled="!canPrev"
          @click="stepDay(-1)"
        />
        <h2
          class="m-0 min-w-0 flex-1 truncate text-center text-label-s font-semibold uppercase tracking-wide text-tx-main first-letter:uppercase"
          aria-live="polite"
        >
          {{ weekdayName(weatherDay) }}
        </h2>
        <ActionButton
          v-if="!soloHoy"
          label=""
          icon="go-next"
          :icon-alt="t('components.WeatherWidget.nextDay')"
          :title="t('components.WeatherWidget.nextDay')"
          variant="ghost"
          size="sm"
          :disabled="!canNext"
          @click="stepDay(1)"
        />
      </div>

      <!-- El resumen del día: icono, temperatura grande, condición y máx./mín.
           En una celda baja va en fila; con alto, apilado y centrado. -->
      <div
        v-if="day"
        class="flex min-w-0 items-center gap-[3cqmin]"
        :class="compactRow ? 'flex-1' : 'flex-col justify-center text-center'"
      >
        <ThemeIcon
          :name="weatherIcon(day.code, day.isDay)"
          class="shrink-0"
          :style="compactRow ? 'width: 26cqmin; height: 26cqmin' : 'width: 22cqmin; height: 22cqmin'"
        />
        <div class="flex min-w-0 flex-col" :class="compactRow ? 'items-start' : 'items-center'">
          <div class="font-bold leading-none tabular-nums text-tx-main" style="font-size: clamp(1.1rem, 18cqmin, 2.4rem)">
            {{ tempText(day.temperature) }}
          </div>
          <div v-if="describe(day.code)" class="truncate text-label-xs text-tx-main first-letter:uppercase">
            {{ describe(day.code) }}
          </div>
          <div class="flex items-center gap-[2cqmin] text-label-xs text-tx-muted tabular-nums">
            <span class="font-semibold text-tx-main">{{ tempText(day.max) }}</span>
            <span>{{ tempText(day.min) }}</span>
          </div>
        </div>
      </div>

      <!-- Los cuatro anillos, cuando hay alto para ellos. -->
      <div
        v-if="day && showRings"
        class="grid min-w-0 gap-[2cqmin]"
        :style="`grid-template-columns: repeat(${ringColumns}, minmax(0, 1fr))`"
      >
        <ProgressRing
          v-for="ring in rings"
          :key="ring.key"
          :value="ring.value"
          :label="ring.label"
          :display="ring.display"
          :unit="ring.unit"
          size="sm"
        />
      </div>

      <!-- Las tarjetas pasantes: los otros días, para elegir uno. El control
           segmentado de la librería (variante de pastillas), que se desplaza sin
           cortar ninguna. -->
      <div v-if="showStrip" class="min-w-0 overflow-x-auto pb-[1cqmin]">
        <SegmentedControl
          variant="chips"
          :options="dayOptions"
          :model-value="weatherDay"
          :label="t('components.WeatherWidget.nextDay')"
          @change="pick"
        />
      </div>
    </div>
  </div>
</template>
