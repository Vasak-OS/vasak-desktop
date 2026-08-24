<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed } from 'vue';
import DailyWeatherCard from '@/components/cards/DailyWeatherCard.vue';
import WeatherIcon from '@/components/icon/WeatherIcon.vue';
import { useWeather } from '@/tools/composables/useWeather';

const { t } = useI18n();

const props = defineProps<{
	/** `extended` es el pronóstico de la semana; `today`, sólo el día de hoy. */
	variant?: string;
}>();

// Los datos no son de este componente: los pide una sola ventana y los comparte
// el cache de Rust. Ver useWeather.
const { weather, current, failed, dayOrNight, upcoming } = useWeather();

const soloHoy = computed(() => props.variant === 'today');

/**
 * Si hay con qué dibujar.
 *
 * No alcanza con «llegó algo»: lo guardado puede ser de una respuesta a medias
 * —o de una versión anterior del servicio— y las dos variantes leen `current`,
 * `current_units` y `daily`. Preguntando sólo por `weather` bastaba un
 * pronóstico raro para que el widget reventara al dibujarse en vez de decir que
 * no hay datos.
 */
const listo = computed(
	() =>
		Boolean(current.value) &&
		Boolean(weather.value?.current_units?.temperature_2m) &&
		Array.isArray(weather.value?.daily?.time) &&
		weather.value.daily.time.length > 0
);

/**
 * `2026-08-08` parseado por `new Date()` es medianoche UTC, así que en cualquier
 * lugar detrás de UTC se dibuja como el día anterior. Armar la fecha por partes
 * la mantiene local en todas las zonas.
 */
const nombreDelDia = (fecha: string) => {
	const [ano, mes, dia] = fecha.split('-').map(Number);
	return new Date(ano, mes - 1, dia).toLocaleDateString(undefined, { weekday: 'long' });
};
</script>

<template>
  <div class="h-full min-h-0 w-full p-[3cqmin]">
    <!-- Sin datos todavía, o sin red: el widget dice qué pasa en vez de quedar
         en blanco. -->
    <div v-if="!listo" class="flex h-full items-center justify-center p-2 text-center text-tx-main/60">
      {{ failed ? t('components.WeatherWidget.failed') : t('components.WeatherWidget.loading') }}
    </div>

    <!-- Variante corta: una sola tarjeta, en fila, para una celda ancha y baja. -->
    <div
      v-else-if="soloHoy"
      class="flex h-full items-center justify-center gap-[5cqmin] rounded-corner border border-primary bg-ui-surface/80 p-[3cqmin]"
    >
      <WeatherIcon
        :code="current.weather_code"
        :dayOrNight="dayOrNight"
        class="shrink-0"
        style="width: 22cqmin; height: 22cqmin"
      />
      <div class="flex min-w-0 flex-col items-start">
        <div class="font-bold leading-none tabular-nums" style="font-size: 26cqmin">
          {{ Math.round(current.temperature_2m) }}{{ weather.current_units.temperature_2m }}
        </div>
        <div class="flex items-center gap-[3cqmin] tabular-nums" style="font-size: 11cqmin">
          <span class="truncate font-semibold text-tx-muted first-letter:uppercase">
            {{ nombreDelDia(weather.daily.time[0]) }}
          </span>
          <span class="font-semibold">{{ Math.round(weather.daily.temperature_2m_max[0]) }}°</span>
          <span class="text-tx-muted">{{ Math.round(weather.daily.temperature_2m_min[0]) }}°</span>
        </div>
      </div>
    </div>

    <!-- Variante extendida: hoy a la izquierda, la semana a la derecha. -->
    <div v-else class="grid h-full min-h-0 gap-[3cqmin]" style="grid-template-columns: 2fr 3fr">
      <div
        class="flex min-h-0 flex-col items-center justify-center gap-[3cqmin] rounded-corner border border-primary bg-ui-surface/80 p-[3cqmin]"
      >
        <div class="font-bold leading-none tabular-nums" style="font-size: 18cqmin">
          {{ Math.round(current.temperature_2m) }}{{ weather.current_units.temperature_2m }}
        </div>
        <WeatherIcon
          :code="current.weather_code"
          :dayOrNight="dayOrNight"
          style="width: 26cqmin; height: 26cqmin"
        />
        <div class="truncate font-semibold first-letter:uppercase" style="font-size: 8cqmin">
          {{ nombreDelDia(weather.daily.time[0]) }}
        </div>
        <div class="flex gap-[3cqmin] tabular-nums" style="font-size: 8cqmin">
          <span class="font-semibold">{{ Math.round(weather.daily.temperature_2m_max[0]) }}°</span>
          <span class="text-tx-muted">{{ Math.round(weather.daily.temperature_2m_min[0]) }}°</span>
        </div>
      </div>

      <!-- Los días que vienen, con el mismo aire entre tarjetas que el que las
           separa del resumen. -->
      <div class="grid min-h-0 grid-cols-3 grid-rows-2 gap-[3cqmin]">
        <DailyWeatherCard
          v-for="dia in upcoming"
          :key="dia.date"
          :date="dia.date"
          :min="dia.min"
          :max="dia.max"
          :units="weather.daily_units"
          :dayOrNight="dayOrNight"
          :weatherCode="dia.code"
          class="h-full min-h-0"
        />
      </div>
    </div>
  </div>
</template>
