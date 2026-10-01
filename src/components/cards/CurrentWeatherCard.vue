<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { computed } from 'vue';
import WeatherIcon from '@/components/icon/WeatherIcon.vue';

const { units, current } = defineProps({
	current: {
		type: Object,
		required: true,
	},
	units: {
		type: Object,
		required: true,
	},
	dayOrNight: {
		type: String,
		required: true,
	},
});

const getTemUnit = computed(() => units.temperature_2m);
const getTemperature = computed(() => current.temperature_2m);
</script>

<template>
  <div class="flex flex-row items-center justify-around border border-primary bg-ui-selected-accent rounded-corner-m p-4 w-full">
    <h3 class="text-2xl font-bold">{{ getTemperature }} {{ getTemUnit }}</h3>
	<WeatherIcon :code="current.weather_code" :dayOrNight="(dayOrNight as 'day' | 'night')" />
  </div>
</template>
