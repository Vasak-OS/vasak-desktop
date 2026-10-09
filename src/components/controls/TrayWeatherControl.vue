<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { PanelPill } from '@vasakgroup/vue-libvasak';
import { computed, ref } from 'vue';
import WeatherIcon from '@/components/icon/WeatherIcon.vue';
import { toggleApplet } from '@/services/window.service';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { usePanelDensity } from '@/tools/composables/usePanelDensity';
import { useWeather } from '@/tools/composables/useWeather';
import { showsNumbers } from '@/tools/panel-density';
import { logError } from '@/utils/logger';

const { t } = useI18n();
const { vertical, hasSurface } = usePanelConfig();
const density = usePanelDensity();

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

/**
 * Tocarla abre el tablero de fecha (vasak-desktop#130), igual que el reloj:
 * ahí está el clima del día con sus anillos y el pronóstico por hora.
 */
const opener = ref<unknown>(null);
const { isOpen } = useOpenApplet('date');
const openLabel = computed(() =>
	detail.value
		? `${t('components.TrayWeatherControl.open')}: ${detail.value}`
		: t('components.TrayWeatherControl.open')
);

async function openBoard(): Promise<void> {
	try {
		await toggleApplet('date', opener.value);
	} catch (error) {
		logError('[TrayWeatherControl] no se pudo abrir el tablero de fecha:', error);
	}
}
</script>

<template>
  <!-- Sin datos no hay lugar reservado: un hueco con un guion al lado del reloj
       es peor que no mostrar nada. En el panel en píldoras (vasak-desktop#151)
       es la píldora del clima, al lado del reloj: el icono y los grados. -->
  <PanelPill
    v-if="current"
    ref="opener"
    :label="vertical || !showsNumbers(density) ? '' : degrees"
    :expanded="isOpen"
    :title="detail"
    :accessible-label="openLabel"
    aria-haspopup="dialog"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    :flat="hasSurface"
    class="shrink-0"
    data-weather-pill
    @click="openBoard"
  >
    <template #leading>
      <WeatherIcon :code="current.weather_code" :dayOrNight="dayOrNight" size-class="h-5 w-5" />
    </template>
  </PanelPill>
</template>
