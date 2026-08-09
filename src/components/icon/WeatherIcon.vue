<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, type Ref, ref } from 'vue';
import { useIcon } from '@/tools/composables/useReactiveIcon';
import weatherCodesData from '@/data/weatherCodes.json';
import type { CodeDataType, WeatherInfo } from '@/interfaces/weather';

const { t } = useI18n();

const codeData: CodeDataType = weatherCodesData as CodeDataType;

const weatherInfo: Ref<WeatherInfo | null> = ref(null);
const props = defineProps<{
	code: number;
	dayOrNight: 'day' | 'night';
}>();

const iconPath = useIcon(computed(() => {
	weatherInfo.value = codeData[String(props.code)];
	if (weatherInfo.value) {
		return weatherInfo.value[props.dayOrNight].image;
	}
	return 'weather-severe-alert';
}));
</script>

<template>
  <transition enter-active-class="transition-opacity duration-300 ease-in-out" leave-active-class="transition-opacity duration-300 ease-in-out" enter-from-class="opacity-0" leave-to-class="opacity-0" mode="out-in">
	<img
	  v-if="iconPath"
	  :src="iconPath"
	  :alt="weatherInfo ? weatherInfo[dayOrNight].description : t('components.WeatherIcon.unknown')"
	  :title="weatherInfo ? weatherInfo[dayOrNight].description : t('components.WeatherIcon.unknown')"
	  class="img-fluid h-16 w-16"
	/>
  </transition>
</template>

