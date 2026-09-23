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
	  <!-- `size="auto"` y no un número: con un número, `ThemeIcon` escribe el
	       alto y el ancho **en línea**, y una regla en línea le gana a
	       `h-full w-full`. Por eso el dibujo se quedaba en 64 píxeles adentro de
	       una caja de 22cqmin —o de `h-5` en la bandeja— y se salía. Con `auto`
	       no escribe nada y el tamaño sale de la caja, que es la única forma de
	       dibujar un icono que se mide en unidades de contenedor.
	       Necesita `@vasakgroup/vue-libvasak` 1.5.0. -->
	  <ThemeIcon :name="iconName" size="auto" :alt="descripcion" class="h-full w-full" />
	</span>
  </transition>
</template>

