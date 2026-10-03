<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { PanelPill } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import WeatherIcon from '@/components/icon/WeatherIcon.vue';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { useWeather } from '@/tools/composables/useWeather';

const { t } = useI18n();
const { vertical } = usePanelConfig();

// El mismo pronóstico que el widget del escritorio: el pedido lo hace una sola
// ventana y el cache de Rust se lo pasa a las demás. Ver useWeather.
const { weather, current, dayOrNight } = useWeather();

const degrees = computed(() =>
	current.value ? `${Math.round(current.value.temperature_2m)}°` : ''
);

/**
 * El panel muestra el número y nada más; el detalle va en el título, que es
 * donde se puede leer sin ocupar la única franja de pantalla que está siempre
 * a la vista.
 */
const detail = computed(() => {
	const daily = weather.value?.daily;
	if (!daily) return '';

	return t('components.TrayWeatherControl.summary')
		.replace('{0}', String(Math.round(daily.temperature_2m_max[0])))
		.replace('{1}', String(Math.round(daily.temperature_2m_min[0])));
});
</script>

<template>
  <!-- Sin datos no hay lugar reservado: un hueco con un guion al lado del reloj
       es peor que no mostrar nada. En el panel en píldoras (vasak-desktop#151)
       es la píldora del clima, al lado del reloj: el icono y los degrees. -->
  <PanelPill
    v-if="current"
    :label="vertical ? '' : degrees"
    :interactive="false"
    :title="detail"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    data-weather-pill
  >
    <template #leading>
      <WeatherIcon :code="current.weather_code" :dayOrNight="dayOrNight" size-class="h-5 w-5" />
    </template>
  </PanelPill>
</template>
