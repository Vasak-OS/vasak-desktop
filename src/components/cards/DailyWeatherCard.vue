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
	// `2026-08-08` parses as UTC midnight, which renders as the previous day
	// anywhere behind UTC; the old +1 day compensated for that but overshot for
	// anyone ahead of UTC. Building from the parts keeps it local everywhere.
	const [year, month, day] = props.date.split('-').map(Number);
	return new Date(year, month - 1, day).toLocaleDateString(undefined, {
		month: 'numeric',
		day: 'numeric',
	});
});

const dayOrNightType = computed(() => props.dayOrNight as 'day' | 'night');
</script>
<template>
  <div class="flex flex-col items-center justify-center gap-1 rounded-corner-m border border-ui-line p-2">
    <div class="text-label-m font-medium">{{ formattedDate }}</div>
	<WeatherIcon :code="weatherCode" :dayOrNight="dayOrNightType" class="weather-icon my-[0.1rem]" />
    <div class="flex gap-2 text-label-m">
      <span class="font-semibold">{{ max }}°</span>
      <span class="font-normal">{{ min }}°</span>
    </div>
  </div>
</template>

