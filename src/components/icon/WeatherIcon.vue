<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, type Ref, ref } from 'vue';
import weatherCodesData from '@/data/weatherCodes.json';
import type { CodeDataType, WeatherInfo } from '@/interfaces/weather';

const { t } = useI18n();

const codeData: CodeDataType = weatherCodesData as CodeDataType;

const weatherInfo: Ref<WeatherInfo | null> = ref(null);
const props = defineProps<{
	code: number;
	dayOrNight: 'day' | 'night';
	/**
	 * El tamaño, como clases de Tailwind.
	 *
	 * Va como propiedad y no como clase del lado de quien lo usa porque Vue
	 * suma las dos clases —la de acá y la de afuera— y cuál gana lo decide el
	 * orden del CSS, no el del atributo: `h-16` le ganaba a `h-5` y el icono
	 * salía de 64 píxeles en un panel de 36.
	 */
	sizeClass?: string;
}>();

const iconName = computed(() => {
	weatherInfo.value = codeData[String(props.code)];
	if (weatherInfo.value) {
		return weatherInfo.value[props.dayOrNight].image;
	}
	return 'weather-severe-alert';
});

const descripcion = computed(() =>
	weatherInfo.value
		? weatherInfo.value[props.dayOrNight].description
		: t('components.WeatherIcon.unknown')
);
</script>

<template>
  <transition enter-active-class="transition-opacity duration-300 ease-in-out" leave-active-class="transition-opacity duration-300 ease-in-out" enter-from-class="opacity-0" leave-to-class="opacity-0" mode="out-in">
	<!-- El `title` va en un envoltorio y no en el icono: `strictTemplates` no
	     acepta atributos sueltos sobre un componente, y un tooltip sobre el
	     dibujo tapa menos que sobre toda la caja. -->
	<span :title="descripcion" :class="['img-fluid inline-flex', props.sizeClass ?? 'h-16 w-16']">
	  <ThemeIcon :name="iconName" :size="64" :alt="descripcion" class="h-full w-full" />
	</span>
  </transition>
</template>

