<script lang="ts" setup>
import { onMounted, Ref, ref } from 'vue';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import type { WeatherInfo, CodeDataType } from '@/interfaces/weather';
import weatherCodesData from '@/data/weatherCodes.json';

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
  <transition name="icon-fade" mode="out-in">
	<img
	  v-if="iconPath"
	  :src="iconPath"
	  :alt="weatherInfo ? weatherInfo[dayOrNight].description : 'Unknown weather condition'"
	  :title="weatherInfo ? weatherInfo[dayOrNight].description : 'Unknown weather condition'"
	  class="img-fluid weather-icon-img h-24"
	/>
  </transition>
</template>

<style scoped>
.weather-icon-img {
  transition: transform 0.3s ease;
}

/* Transición para cambio de iconos del clima */
.icon-fade-enter-active,
.icon-fade-leave-active {
  transition: opacity 0.3s ease;
}

.icon-fade-enter-from,
.icon-fade-leave-to {
  opacity: 0;
}
</style>
