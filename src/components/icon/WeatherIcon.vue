<script lang="ts" setup>
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { onMounted, type Ref, ref } from 'vue';
import weatherCodesData from '@/data/weatherCodes.json';
import type { CodeDataType, WeatherInfo } from '@/interfaces/weather';

const codeData: CodeDataType = weatherCodesData as CodeDataType;

const iconPath: Ref<string> = ref('');
const weatherInfo: Ref<WeatherInfo | null> = ref(null);
const props = defineProps<{
	code: number;
	dayOrNight: 'day' | 'night';
}>();

onMounted(async () => {
	weatherInfo.value = codeData[String(props.code)];
	if (weatherInfo.value) {
		iconPath.value = await getIconSource(weatherInfo.value[props.dayOrNight].image);
	} else {
		iconPath.value = 'weather-severe-alert'; // Default icon for unknown codes
	}
});
</script>

<template>
  <transition enter-active-class="transition-opacity duration-300 ease-in-out" leave-active-class="transition-opacity duration-300 ease-in-out" enter-from-class="opacity-0" leave-to-class="opacity-0" mode="out-in">
	<img
	  v-if="iconPath"
	  :src="iconPath"
	  :alt="weatherInfo ? weatherInfo[dayOrNight].description : 'Unknown weather condition'"
	  :title="weatherInfo ? weatherInfo[dayOrNight].description : 'Unknown weather condition'"
	  class="img-fluid weather-icon-img h-24"
	/>
  </transition>
</template>

