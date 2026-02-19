<script lang="ts" setup>
import { computed } from 'vue';

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

const _getTemUnit = computed(() => units.temperature_2m);
const _getTemperature = computed(() => current.temperature_2m);
</script>

<template>
  <div class="flex flex-row items-center justify-around">
    <h3 class="text-2xl font-bold">{{ getTemperature }} {{ getTemUnit }}</h3>
	<WeatherIcon :code="current.weather_code" :dayOrNight="(dayOrNight as 'day' | 'night')" />
  </div>
</template>
