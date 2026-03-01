<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { computed } from 'vue';
import WeatherIcon from '@/components/icon/WeatherIcon.vue';

const props = defineProps({
	date: {
		type: String,
		required: true,
	},
	min: {
		type: Number,
		required: true,
	},
	max: {
		type: Number,
		required: true,
	},
	units: {
		type: Object,
		required: true,
	},
	dayOrNight: {
		type: String as () => 'day' | 'night',
		required: true,
	},
	weatherCode: {
		type: Number,
		required: true,
	},
});

const formattedDate = computed(() => {
	const dateObj = new Date(props.date);
	dateObj.setDate(dateObj.getDate() + 1);
	return dateObj.toLocaleDateString(undefined, { month: 'numeric', day: 'numeric' });
});

const dayOrNightType = computed(() => props.dayOrNight as 'day' | 'night');
</script>
<template>
  <div class="flex flex-col items-center gap-1 p-2 rounded-corner bg-primary/80 dark:bg-primary-dark/80 group transition-all duration-200 ease-out hover:-translate-y-1 hover:scale-[1.02] hover:shadow-[0_3px_10px_rgba(0,0,0,0.15)]">
    <div class="text-sm font-medium">{{ formattedDate }}</div>
	<WeatherIcon :code="weatherCode" :dayOrNight="dayOrNightType" class="weather-icon my-[0.1rem] transition-transform duration-200 ease-in-out group-hover:scale-110" />
    <div class="flex gap-2 text-sm">
      <span class="font-semibold">{{ max }}°</span>
      <span class="font-normal">{{ min }}°</span>
    </div>
  </div>
</template>

