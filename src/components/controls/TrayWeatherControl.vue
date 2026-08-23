<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed } from 'vue';
import WeatherIcon from '@/components/icon/WeatherIcon.vue';
import { useWeather } from '@/tools/composables/useWeather';

const { t } = useI18n();

// El mismo pronóstico que el widget del escritorio: el pedido lo hace una sola
// ventana y el cache de Rust se lo pasa a las demás. Ver useWeather.
const { weather, current, dayOrNight } = useWeather();

const grados = computed(() =>
	current.value ? `${Math.round(current.value.temperature_2m)}°` : ''
);

/**
 * El panel muestra el número y nada más; el detalle va en el título, que es
 * donde se puede leer sin ocupar la única franja de pantalla que está siempre
 * a la vista.
 */
const detalle = computed(() => {
	const diario = weather.value?.daily;
	if (!diario) return '';

	return t('components.TrayWeatherControl.summary')
		.replace('{0}', String(Math.round(diario.temperature_2m_max[0])))
		.replace('{1}', String(Math.round(diario.temperature_2m_min[0])));
});
</script>

<template>
  <!-- Sin datos no hay lugar reservado: un hueco con un guion al lado del reloj
       es peor que no mostrar nada. -->
  <div
    v-if="current"
    class="flex items-center gap-1 rounded-corner p-1"
    :title="detalle"
  >
    <WeatherIcon :code="current.weather_code" :dayOrNight="dayOrNight" size-class="h-5 w-5" />
    <span class="text-xs font-semibold tabular-nums text-tx-main">{{ grados }}</span>
  </div>
</template>
